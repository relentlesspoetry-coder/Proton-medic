//! Explicit, run-scoped shutdown requests for the independent runtime supervisor.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="camelCase")]
pub struct Session { pub id:String, pub run_id:String, pub name:String, pub kind:String, pub can_stop:bool }
fn logs(dir:&Path,run:&str)->Result<PathBuf,String>{
    if !valid_id(run){return Err("Invalid run identity".into());}
    let parent=dir.join("logs");directory(&parent,false)?;
    let path=parent.join(run);directory(&path,false)?;Ok(path)
}
pub fn active()->Result<Vec<Session>,String>{active_in(&root()?)}
fn active_in(root:&Path)->Result<Vec<Session>,String>{
    let mut sessions=vec![];
    for item in fs::read_dir(root).map_err(|e|e.to_string())? {
        let item=item.map_err(|e|e.to_string())?;let id=item.file_name().to_string_lossy().into_owned();
        if !valid_id(&id){continue;}
        let Ok((dir,r))=load(root,&id) else {continue;};
        if !r.state.ends_with("running") || r.boot_id.is_none() || r.boot_id!=boot_id(){continue;}
        // A stale record is not authority to stop anything.
        if lock_profile(&dir).is_ok(){continue;}
        let Some(run)=r.run_id else{continue;};
        let can_stop=logs(&dir,&run).ok().is_some_and(|path|{
            let marker=path.join("stop-supported");
            regular(&marker,80).is_ok() && fs::read_to_string(marker).is_ok_and(|s|s==run)
        });
        sessions.push(Session{id:r.id,run_id:run,name:r.name,kind:if r.state=="installer-running"{"installer"}else{"game"}.into(),can_stop});
    }
    sessions.sort_by(|a,b|a.id.cmp(&b.id));Ok(sessions)
}
fn validate(expected:&[Session],current:&[Session],confirmed:bool)->Result<(),String>{
    if !confirmed{return Err("Approve stopping the listed games and installers first".into());}
    if expected.is_empty() || expected.len()>16{return Err("Invalid session selection".into());}
    let mut seen=HashSet::new();
    for session in expected {
        if !seen.insert(&session.id) || !session.can_stop || !current.contains(session){
            return Err("The active sessions changed. Review them again before stopping anything.".into());
        }
    }
    Ok(())
}
pub fn stop(expected:Vec<Session>,confirmed:bool)->Result<(),String>{
    let root=root()?;validate(&expected,&active_in(&root)?,confirmed)?;
    for session in &expected {
        let (dir,r)=load(&root,&session.id)?;
        if r.run_id.as_deref()!=Some(&session.run_id){return Err("The active session changed. Review it again.".into());}
        let path=logs(&dir,&session.run_id)?.join("stop-request");
        match OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
            Ok(mut file)=>{file.write_all(session.run_id.as_bytes()).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;},
            Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},
            Err(e)=>return Err(e.to_string()),
        }
    }
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(15);
    loop {
        let mut pending=false;
        for session in &expected {
            let (dir,record)=load(&root,&session.id)?;
            if record.run_id.as_deref()==Some(&session.run_id) {
                let local_busy=JOBS.lock().map_err(|_|"Process registry unavailable")?.contains(&session.id);
                pending |= !released(&dir,local_busy);
            }
        }
        if !pending{return Ok(());}
        if std::time::Instant::now()>=deadline{return Err("A game or installer is still closing. Close it manually, then retry the update. Nothing was force-killed.".into());}
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
fn released(dir:&Path,local_busy:bool)->bool { !local_busy && lock_profile(dir).is_ok() }
#[cfg(test)] mod tests {
    use super::*;
    fn session(run:&str)->Session{Session{id:"1-2".into(),run_id:run.into(),name:"Game".into(),kind:"game".into(),can_stop:true}}
    #[test] fn shutdown_requires_consent_and_exact_reviewed_run(){
        let a=session("1-3");let b=session("1-4");
        assert!(validate(&[a.clone()],&[a.clone()],false).is_err());
        assert!(validate(&[a.clone()],&[b],true).is_err());
        assert!(validate(&[a.clone(),a.clone()],&[a.clone()],true).is_err());
        assert!(validate(&[a.clone()],&[a],true).is_ok());
    }
    #[test] fn unreadable_inactive_profiles_do_not_block_update_preview(){
        let path=std::env::temp_dir().join(format!("pm-stop-preview-{}",stamp().unwrap()));
        fs::create_dir(&path).unwrap();fs::create_dir(path.join("1-2")).unwrap();
        fs::write(path.join("1-2/profile.json"),b"invalid").unwrap();
        assert!(active_in(&path).unwrap().is_empty());fs::remove_dir_all(path).unwrap();
    }
    #[test] fn completion_waits_for_supervisor_lease_and_parent_registry(){
        let path=std::env::temp_dir().join(format!("pm-stop-complete-{}",stamp().unwrap()));fs::create_dir(&path).unwrap();
        let lease=lock_profile(&path).unwrap();assert!(!released(&path,false));
        drop(lease);assert!(!released(&path,true));assert!(released(&path,false));fs::remove_dir_all(path).unwrap();
    }
    #[test] fn older_supervisors_cannot_be_stopped(){let mut a=session("1-3");a.can_stop=false;assert!(validate(&[a.clone()],&[a],true).is_err());}
}
