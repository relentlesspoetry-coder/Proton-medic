//! Bounded runtime output and Linux process-family observation. Never kills a process.
use std::{collections::{HashMap, HashSet}, fs::{self, File}, io::{self, Read, Seek, SeekFrom, Write}, os::fd::AsRawFd,
    process::{Child, ChildStderr, ChildStdout, ExitStatus}, sync::{Arc, atomic::{AtomicBool, Ordering}}, thread,
    time::{Duration, Instant}};
const LIMIT: usize = 2 * 1024 * 1024;
struct Tail { file: File, cursor: usize, len: usize }
impl Tail {
    fn new(file: File) -> Self { Self { file, cursor: 0, len: 0 } }
    fn write(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            let n = bytes.len().min(LIMIT - self.cursor);
            self.file.seek(SeekFrom::Start(self.cursor as u64))?;
            self.file.write_all(&bytes[..n])?;
            self.cursor = (self.cursor + n) % LIMIT;
            self.len = (self.len + n).min(LIMIT);
            bytes = &bytes[n..];
        }
        Ok(())
    }
    fn finish(mut self) -> io::Result<()> {
        if self.len == LIMIT && self.cursor != 0 {
            // One chronological rewrite at exit, rather than repeated full-tail rewrites.
            let mut bytes = vec![0; LIMIT];
            self.file.seek(SeekFrom::Start(0))?; self.file.read_exact(&mut bytes)?;
            bytes.rotate_left(self.cursor);
            self.file.seek(SeekFrom::Start(0))?; self.file.write_all(&bytes)?;
        }
        self.file.set_len(self.len as u64)?; self.file.sync_all()
    }
}
#[cfg(test)]
pub fn capture_fixture(mut input: impl Read, output: File) -> io::Result<()> {
    let mut tail = Tail::new(output); let mut buf = [0; 8192];
    loop { let n = input.read(&mut buf)?; if n == 0 { break; } tail.write(&buf[..n])?; }
    tail.finish()
}
fn capture(mut input: impl Read + AsRawFd, output: File, stop: Arc<AtomicBool>) -> io::Result<()> {
    let fd = input.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 { return Err(io::Error::last_os_error()); }
    let mut tail = Tail::new(output); let mut buf = [0; 8192]; let mut ending = None;
    loop {
        if stop.load(Ordering::Acquire) {
            let at = ending.get_or_insert_with(Instant::now);
            if at.elapsed() >= Duration::from_millis(250) { break; }
        }
        match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => tail.write(&buf[..n])?,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                if ending.is_some() { break; }
                thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(e),
        }
    }
    tail.finish()
}
#[derive(Clone)]
pub(crate) struct Process { pub(crate) pid: u32, pub(crate) parent: u32, pub(crate) group: u32, pub(crate) start: u64, pub(crate) state: char, pub(crate) metrics: Option<(u64,u64)> }
fn parse_process(pid: u32, stat: &str) -> Option<Process> {
    // comm may contain spaces and parentheses. Fields after its final ')' are fixed.
    let fields: Vec<_> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    Some(Process { pid, state: fields.first()?.chars().next()?, parent: fields.get(1)?.parse().ok()?,
        group: fields.get(2)?.parse().ok()?, start: fields.get(19)?.parse().ok()?,
        metrics: fields.get(11).and_then(|s|s.parse::<u64>().ok()).zip(fields.get(12).and_then(|s|s.parse::<u64>().ok()))
            .zip(fields.get(21).and_then(|s|s.parse::<u64>().ok())).map(|((user,kernel),rss)|(user.saturating_add(kernel),rss)) })
}
fn processes() -> io::Result<Vec<Process>> {
    let mut result = vec![];
    for item in fs::read_dir("/proc")?.flatten() {
        let Some(pid) = item.file_name().to_str().and_then(|s| s.parse().ok()) else { continue };
        match fs::read_to_string(item.path().join("stat")) {
            Ok(stat) => { if let Some(p) = parse_process(pid, &stat) { result.push(p); } },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {},
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {},
            Err(e) => return Err(e),
        }
    }
    Ok(result)
}
fn family_alive(all: &[Process], group: u32, known: &mut HashMap<u32, u64>) -> bool {
    known.retain(|pid, start| all.iter().any(|p| p.pid == *pid && p.start == *start));
    let mut family: HashSet<u32> = all.iter().filter(|p| p.group == group || known.get(&p.pid) == Some(&p.start)).map(|p| p.pid).collect();
    loop {
        let before = family.len();
        for p in all { if family.contains(&p.parent) { family.insert(p.pid); } }
        if family.len() == before { break; }
    }
    for p in all.iter().filter(|p| family.contains(&p.pid)) { known.insert(p.pid, p.start); }
    all.iter().any(|p| family.contains(&p.pid) && !matches!(p.state, 'Z' | 'X'))
}
#[cfg(test)]
pub fn supervise(child: Child, out: ChildStdout, err: ChildStderr, stdout: File, stderr: File) -> (io::Result<ExitStatus>, bool) {
    supervise_monitored(child,out,err,stdout,stderr,None)
}
pub fn supervise_monitored(mut child: Child, out: ChildStdout, err: ChildStderr, stdout: File, stderr: File, logs: Option<&std::path::Path>) -> (io::Result<ExitStatus>, bool) {
    let mut monitor=logs.map(crate::windows_games::monitor::Sampler::new);
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone(); let a = thread::spawn(move || capture(out, stdout, signal));
    let signal = stop.clone(); let b = thread::spawn(move || capture(err, stderr, signal));
    let group = child.id(); let mut known = HashMap::new(); let mut status = None;
    loop {
        if status.is_none() {
            match child.try_wait() { Ok(Some(s)) => status = Some(Ok(s)), Ok(None) => {}, Err(e) => status = Some(Err(e)) }
        }
        // The process group survives parent exit. Remember observed descendants that change groups.
        // If /proc cannot be inspected, keep the profile locked and retry rather than assume it idle.
        if let Ok(all) = processes() {
            let alive=family_alive(&all, group, &mut known);
            if let Some(monitor)=monitor.as_mut(){monitor.observe(&all,&known);}
            if !alive && status.is_some() { break; }
        }
        thread::sleep(Duration::from_millis(200));
    }
    stop.store(true, Ordering::Release);
    let captures = [a.join(), b.join()].into_iter().all(|r| matches!(r, Ok(Ok(()))));
    (status.unwrap(), captures)
}
#[cfg(test)] mod tests {
    use super::*;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    #[test] fn descendant_outlives_launcher_without_releasing_session_early() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "sleep 0.7 & exit 0"]).process_group(0).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().unwrap(); let out = child.stdout.take().unwrap(); let err = child.stderr.take().unwrap();
        let at = Instant::now();
        let (status, captured) = supervise(child, out, err, tempfile("out"), tempfile("err"));
        assert!(status.unwrap().success()); assert!(captured); assert!(at.elapsed() >= Duration::from_millis(600));
        assert!(at.elapsed() < Duration::from_secs(5));
    }
    fn tempfile(label: &str) -> File {
        let path = std::env::temp_dir().join(format!("pm-capture-{}-{label}-{:?}",std::process::id(),Instant::now()));
        let file = std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(&path).unwrap();
        fs::remove_file(path).unwrap(); file
    }
    #[test] fn pipe_can_be_closed_without_waiting_for_an_abandoned_writer() {
        let mut child = Command::new("sh").args(["-c", "sleep 0.8"]).stdout(Stdio::piped()).spawn().unwrap();
        let out = child.stdout.take().unwrap(); let at = Instant::now();
        capture(out, tempfile("abandoned"), Arc::new(AtomicBool::new(true))).unwrap();
        assert!(at.elapsed() < Duration::from_millis(500)); assert!(child.wait().unwrap().success());
    }
    #[test] fn pid_reuse_does_not_keep_an_unrelated_process_in_the_family() {
        let mut known = HashMap::from([(12, 1)]);
        let all = vec![Process {pid:12,parent:1,group:99,start:2,state:'S',metrics:None}];
        assert!(!family_alive(&all, 50, &mut known)); assert!(known.is_empty());
    }
    #[test] fn observed_descendants_remain_tracked_after_reparenting_and_setsid() {
        let mut known = HashMap::from([(12, 1)]);
        let all = vec![Process {pid:12,parent:1,group:99,start:1,state:'S',metrics:None},Process {pid:13,parent:12,group:99,start:2,state:'S',metrics:None}];
        assert!(family_alive(&all, 50, &mut known)); assert_eq!(known.get(&13),Some(&2));
        let zombies = vec![Process {pid:12,parent:1,group:99,start:1,state:'Z',metrics:None}];
        assert!(!family_alive(&zombies,50,&mut known));
    }
    #[test] fn proc_stat_parsing_uses_own_cpu_ticks_and_resident_pages() {
        let mut fields=vec!["0";22];fields[0]="S";fields[1]="1";fields[2]="50";fields[11]="120";fields[12]="30";fields[13]="9000";fields[14]="8000";fields[19]="12345";fields[21]="256";
        let process=parse_process(50,&format!("50 (game (worker)) {}",fields.join(" "))).unwrap();
        assert_eq!(process.start,12345);assert_eq!(process.metrics,Some((150,256)));
    }
    #[test] fn ring_retains_chronological_tail_across_multiple_wraps() {
        let mut tail = Tail::new(tempfile("ring"));
        tail.write(&vec![b'a';LIMIT+17]).unwrap(); tail.write(&vec![b'b';LIMIT-31]).unwrap();
        let mut read = tail.file.try_clone().unwrap(); tail.finish().unwrap(); read.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes=vec![];read.read_to_end(&mut bytes).unwrap(); assert_eq!(bytes.len(),LIMIT);
        assert_eq!(&bytes[..31],&[b'a';31]); assert!(bytes[31..].iter().all(|b|*b==b'b'));
    }
}
