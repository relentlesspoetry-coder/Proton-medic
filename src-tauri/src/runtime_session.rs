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

// A dedicated process adopts orphaned descendants before the runtime starts.
// Unlike process-group polling, this also covers double-fork/setsid launchers.
#[derive(serde::Serialize, serde::Deserialize)]
struct LaunchSpec {
    program: std::ffi::OsString,
    args: Vec<std::ffi::OsString>,
    env: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)>,
    cwd: Option<std::path::PathBuf>,
}
pub(crate) fn spawn_runtime(command: &mut std::process::Command) -> io::Result<Child> {
    // Unit fixtures exercise command construction; packaged-binary tests exercise re-exec.
    #[cfg(test)] { return command.spawn(); }
    #[cfg(not(test))] {
        use std::os::unix::process::CommandExt;
        fs::metadata(command.get_program())?;
        let spec = LaunchSpec {
            program: command.get_program().into(), args: command.get_args().map(Into::into).collect(),
            env: command.get_envs().map(|(k,v)|(k.into(),v.map(Into::into))).collect(),
            cwd: command.get_current_dir().map(Into::into),
        };
        // Inherit bundle libraries for our helper; apply host overrides only to the runtime.
        std::process::Command::new(std::env::current_exe()?)
            .arg("--supervise-runtime").arg(serde_json::to_string(&spec)?)
            .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped()).process_group(0).spawn()
    }
}
pub fn handle_cli() -> Option<i32> {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new("--supervise-runtime")) { return None; }
    let result = (|| -> Result<i32, Box<dyn std::error::Error>> {
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() != 3 { return Err("Invalid supervisor arguments".into()); }
        let spec: LaunchSpec = serde_json::from_str(args[2].to_str().ok_or("Invalid launch specification")?)?;
        if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong) } != 0 { return Err(io::Error::last_os_error().into()); }
        let mut command = std::process::Command::new(spec.program);
        command.args(spec.args);
        for (key,value) in spec.env { if let Some(value)=value { command.env(key,value); } else { command.env_remove(key); } }
        if let Some(cwd)=spec.cwd { command.current_dir(cwd); }
        let child = command.spawn()?;
        let primary = child.id() as libc::pid_t;
        let mut exit_code = 125;
        loop {
            let mut status = 0;
            let pid = unsafe { libc::waitpid(-1, &mut status, 0) };
            if pid < 0 {
                let error=io::Error::last_os_error();
                if error.raw_os_error()==Some(libc::EINTR) { continue; }
                if error.raw_os_error()==Some(libc::ECHILD) { break; }
                return Err(error.into());
            }
            if pid == primary {
                exit_code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) }
                    else { 128 + libc::WTERMSIG(status) };
            }
        }
        Ok(exit_code)
    })();
    Some(match result { Ok(code)=>code, Err(error)=>{ eprintln!("Runtime supervisor: {error}");125 } })
}
