//! Run-scoped shutdown requests. Never signal names, prefixes, or process groups.
use super::*;
use std::{os::{fd::FromRawFd,unix::fs::OpenOptionsExt},path::{Path,PathBuf}};
const FORCE_GRACE: Duration = Duration::from_secs(2);
pub(super) struct StopControl {
    logs:PathBuf, run:String, sent:HashSet<(u32,u64)>, force_started:Option<Instant>,
}
impl StopControl {
    pub(super) fn new(logs:&Path)->Self {
        let run=logs.file_name().unwrap_or_default().to_string_lossy().into_owned();
        for name in ["stop-supported", "force-stop-supported"] {
            if let Ok(mut file)=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(logs.join(name)) {
                let _=file.write_all(run.as_bytes());
            }
        }
        Self{logs:logs.into(),run,sent:HashSet::new(),force_started:None}
    }
    fn requested(&self,name:&str)->bool {
        let request=self.logs.join(name);
        fs::symlink_metadata(&request).is_ok_and(|m|m.is_file()&&!m.file_type().is_symlink()&&m.len()<=80)
            && fs::read_to_string(request).is_ok_and(|s|s==self.run)
    }
    pub(super) fn poll(&mut self,all:&[Process]) {
        if self.requested("force-stop-request") { self.force_started.get_or_insert_with(Instant::now); }
        if self.force_started.is_none() && !self.requested("stop-request") {return;}
        let force=self.force_started.is_some_and(|at|at.elapsed()>=FORCE_GRACE);
        // The dedicated subreaper adopts orphaned/double-forked children. Rescan on
        // every poll, including after KILL: a process may fork during either phase.
        // waitpid(ECHILD), not a snapshot, is the final completion authority.
        for process in descendants(all,std::process::id()) {
            let key=(process.pid,process.start);
            if force { let _=signal(process,libc::SIGKILL); }
            else if !self.sent.contains(&key) && signal(process,libc::SIGTERM).is_ok(){self.sent.insert(key);}
        }
    }
}
fn descendants(all:&[Process],supervisor:u32)->Vec<&Process>{
    let Some(root)=all.iter().find(|p|p.pid==supervisor) else{return vec![];};
    let mut owned=HashMap::from([(supervisor,root.start)]);
    loop {
        let before=owned.len();
        // A recycled parent PID newer than its supposed child cannot establish
        // ancestry in a /proc snapshot assembled while processes are exiting.
        for p in all {if owned.get(&p.parent).is_some_and(|start|*start<=p.start){owned.insert(p.pid,p.start);}}
        if before==owned.len(){break;}
    }
    all.iter().filter(|p|p.pid!=supervisor&&owned.get(&p.pid)==Some(&p.start)&&!matches!(p.state,'Z'|'X')).collect()
}
fn signal(process:&Process,signal:i32)->io::Result<()> {
    // pidfd binds the signal to one process, even if its numeric PID is reused.
    let fd=unsafe{libc::syscall(libc::SYS_pidfd_open,process.pid,0)};
    if fd<0{return Err(io::Error::last_os_error());}
    let fd=unsafe{File::from_raw_fd(fd as i32)};
    let current=fs::read(format!("/proc/{}/stat",process.pid))?;
    if parse_process(process.pid,&current).is_none_or(|p|p.start!=process.start){return Err(io::Error::other("Process identity changed"));}
    if unsafe{libc::syscall(libc::SYS_pidfd_send_signal,fd.as_raw_fd(),signal,std::ptr::null::<libc::siginfo_t>(),0)}<0{return Err(io::Error::last_os_error());}
    Ok(())
}
#[cfg(test)] mod tests {
    use super::*;
    fn p(pid:u32,parent:u32)->Process{Process{pid,parent,group:10,start:1,state:'S',metrics:None}}
    #[test] fn only_descendants_are_selected_not_unrelated_group_members(){
        let all=vec![p(10,1),p(11,10),p(12,11),p(13,1)];
        assert_eq!(descendants(&all,10).iter().map(|p|p.pid).collect::<Vec<_>>(),vec![11,12]);
    }
    #[test] fn recycled_parent_cannot_claim_an_older_child(){
        let mut parent=p(11,10);parent.start=3;
        let all=vec![p(10,1),parent,p(12,11)];
        assert_eq!(descendants(&all,10).iter().map(|p|p.pid).collect::<Vec<_>>(),vec![11]);
    }
    #[test] fn mismatched_identity_cannot_signal_a_process(){
        let mut child=std::process::Command::new("sleep").arg("2").spawn().unwrap();
        let mut observed=parse_process(child.id(),&fs::read(format!("/proc/{}/stat",child.id())).unwrap()).unwrap();
        observed.start=observed.start.wrapping_add(1);
        for sig in [libc::SIGTERM,libc::SIGKILL] {assert!(signal(&observed,sig).is_err());}
        assert!(child.try_wait().unwrap().is_none());child.wait().unwrap();
    }
    #[test] fn force_deadline_kills_term_resistant_child_and_late_descendants(){
        use std::os::unix::process::ExitStatusExt;
        let path=std::env::temp_dir().join(format!("pm-force-{}-{:?}",std::process::id(),Instant::now()));
        fs::create_dir(&path).unwrap();
        let ready=path.join("ready");
        let mut child=std::process::Command::new("sh").args(["-c","trap '' TERM; touch \"$1\"; exec sleep 10","sh"]).arg(&ready).spawn().unwrap();
        let deadline=Instant::now()+Duration::from_secs(5);
        while !ready.exists(){assert!(Instant::now()<deadline);thread::sleep(Duration::from_millis(10));}
        let read=|pid|parse_process(pid,&fs::read(format!("/proc/{pid}/stat")).unwrap()).unwrap();
        let all=vec![read(std::process::id()),read(child.id())];
        let mut control=StopControl::new(&path);
        fs::write(path.join("stop-request"),&control.run).unwrap();
        control.poll(&all);assert!(child.try_wait().unwrap().is_none());
        assert!(control.force_started.is_none());
        fs::write(path.join("force-stop-request"),"wrong-run").unwrap();
        control.poll(&all);assert!(control.force_started.is_none());
        fs::write(path.join("force-stop-request"),&control.run).unwrap();
        control.poll(&all);assert!(control.force_started.is_some());
        control.force_started=Some(Instant::now()-FORCE_GRACE);
        control.poll(&all);assert_eq!(child.wait().unwrap().signal(),Some(libc::SIGKILL));
        let mut late=std::process::Command::new("sleep").arg("10").spawn().unwrap();
        control.poll(&[read(std::process::id()),read(late.id())]);
        assert_eq!(late.wait().unwrap().signal(),Some(libc::SIGKILL));
        fs::remove_dir_all(path).unwrap();
    }
    #[test] fn exact_disposable_process_can_be_terminated(){
        use std::os::unix::process::ExitStatusExt;
        let mut child=std::process::Command::new("sleep").arg("2").spawn().unwrap();
        let observed=parse_process(child.id(),&fs::read(format!("/proc/{}/stat",child.id())).unwrap()).unwrap();
        signal(&observed,libc::SIGTERM).unwrap();assert_eq!(child.wait().unwrap().signal(),Some(libc::SIGTERM));
    }
}
