//! Run-scoped, consent-driven SIGTERM requests. No process-name or group-wide kills.
use super::*;
use std::{os::{fd::FromRawFd,unix::fs::OpenOptionsExt},path::{Path,PathBuf}};
pub(super) struct StopControl { logs:PathBuf, run:String, sent:HashSet<(u32,u64)> }
impl StopControl {
    pub(super) fn new(logs:&Path)->Self {
        let run=logs.file_name().unwrap_or_default().to_string_lossy().into_owned();
        if let Ok(mut file)=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(logs.join("stop-supported")) {
            let _=file.write_all(run.as_bytes());
        }
        Self{logs:logs.into(),run,sent:HashSet::new()}
    }
    pub(super) fn poll(&mut self,all:&[Process]) {
        let request=self.logs.join("stop-request");
        if !fs::symlink_metadata(&request).is_ok_and(|m|m.is_file()&&!m.file_type().is_symlink()&&m.len()<=80){return;}
        if !fs::read_to_string(request).is_ok_and(|s|s==self.run){return;}
        for process in descendants(all,std::process::id()) {
            let key=(process.pid,process.start);
            if !self.sent.contains(&key) && terminate(process).is_ok(){self.sent.insert(key);}
        }
    }
}
fn descendants(all:&[Process],supervisor:u32)->Vec<&Process>{
    let mut owned=HashSet::from([supervisor]);
    loop {let before=owned.len();for p in all {if owned.contains(&p.parent){owned.insert(p.pid);}}if before==owned.len(){break;}}
    all.iter().filter(|p|p.pid!=supervisor&&owned.contains(&p.pid)&&!matches!(p.state,'Z'|'X')).collect()
}
fn terminate(process:&Process)->io::Result<()> {
    // pidfd binds the signal to one process, even if its numeric PID is reused.
    let fd=unsafe{libc::syscall(libc::SYS_pidfd_open,process.pid,0)};
    if fd<0{return Err(io::Error::last_os_error());}
    let fd=unsafe{File::from_raw_fd(fd as i32)};
    let current=fs::read(format!("/proc/{}/stat",process.pid))?;
    if parse_process(process.pid,&current).is_none_or(|p|p.start!=process.start){return Err(io::Error::other("Process identity changed"));}
    if unsafe{libc::syscall(libc::SYS_pidfd_send_signal,fd.as_raw_fd(),libc::SIGTERM,std::ptr::null::<libc::siginfo_t>(),0)}<0{return Err(io::Error::last_os_error());}
    Ok(())
}
#[cfg(test)] mod tests {
    use super::*;
    fn p(pid:u32,parent:u32)->Process{Process{pid,parent,group:10,start:1,state:'S',metrics:None}}
    #[test] fn only_descendants_are_selected_not_unrelated_group_members(){
        let all=vec![p(10,1),p(11,10),p(12,11),p(13,1)];
        assert_eq!(descendants(&all,10).iter().map(|p|p.pid).collect::<Vec<_>>(),vec![11,12]);
    }
    #[test] fn mismatched_identity_cannot_signal_a_process(){
        let mut child=std::process::Command::new("sleep").arg("2").spawn().unwrap();
        let mut observed=parse_process(child.id(),&fs::read(format!("/proc/{}/stat",child.id())).unwrap()).unwrap();
        observed.start=observed.start.wrapping_add(1);
        assert!(terminate(&observed).is_err());assert!(child.try_wait().unwrap().is_none());child.wait().unwrap();
    }
    #[test] fn exact_disposable_process_can_be_terminated(){
        use std::os::unix::process::ExitStatusExt;
        let mut child=std::process::Command::new("sleep").arg("2").spawn().unwrap();
        let observed=parse_process(child.id(),&fs::read(format!("/proc/{}/stat",child.id())).unwrap()).unwrap();
        terminate(&observed).unwrap();assert_eq!(child.wait().unwrap().signal(),Some(libc::SIGTERM));
    }
}
