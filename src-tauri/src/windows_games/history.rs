//! Per-run receipts, read on demand. Existing logs and game files are never removed.
use super::*;
use std::{cmp::Reverse, collections::BinaryHeap};
const LIMIT: usize = 20;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub runtime: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub state: String,
    pub exit_code: Option<i32>,
    pub message: String,
    pub summaries: Vec<String>,
    pub partial_logs: bool,
    boot_id: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History { pub entries: Vec<Entry>, pub warnings: Vec<String> }
fn now() -> Result<u64, String> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis().try_into().map_err(|_|"Invalid system time")?)
}
fn write(logs: &Path, entry: &Entry) -> Result<(), String> {
    directory(logs, false)?;
    let target = logs.join("history.json");
    if target.exists() { regular(&target, 65536)?; }
    let temp = logs.join(format!(".history-{}.tmp", stamp()?));
    let bytes = serde_json::to_vec(entry).map_err(|e|e.to_string())?;
    if bytes.len() > 65536 { return Err("Launch history record is too large".into()); }
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp).map_err(|e|e.to_string())?;
        file.write_all(&bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        fs::rename(&temp, &target).map_err(|e|e.to_string())?;
        File::open(logs).and_then(|f|f.sync_all()).map_err(|e|e.to_string())
    })();
    if result.is_err() { let _ = fs::remove_file(temp); }
    result
}
fn read(logs: &Path, id: &str) -> Result<Entry, String> {
    directory(logs, false)?;
    let path = logs.join("history.json"); regular(&path, 65536)?;
    let entry: Entry = serde_json::from_slice(&fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    if entry.id != id || entry.started_at > 8_640_000_000_000_000 || entry.summaries.len() > 20 { return Err("Invalid launch history record".into()); }
    Ok(entry)
}
pub(super) fn begin(logs: &Path, record: &Record, runtime: &Runtime, install: bool) -> Result<(), String> {
    write(logs, &Entry { id: record.run_id.clone().ok_or("Missing run identity")?, kind: if install {"installer"} else {"game"}.into(),
        runtime: runtime.name.chars().take(200).collect(), started_at: now()?, ended_at: None,
        state: "running".into(), exit_code: None, message: "Runtime exit has not been recorded.".into(),
        summaries: vec![], partial_logs: false, boot_id: record.boot_id.clone() })
}
pub(super) fn finish(dir: &Path, record: &Record) -> Result<(), String> {
    let Some(id) = record.run_id.as_deref().filter(|id|valid_id(id)) else { return Ok(()) };
    let root = dir.join("logs"); directory(&root, false)?;
    let logs = root.join(id);
    // Older profiles have no history metadata; do not invent their start time.
    if fs::symlink_metadata(logs.join("history.json")).is_err_and(|e|e.kind() == std::io::ErrorKind::NotFound) { return Ok(()); }
    let mut entry = read(&logs, id)?;
    if entry.ended_at.is_some() { return Ok(()); }
    entry.ended_at = Some(now()?); entry.state = record.state.clone(); entry.exit_code = record.exit_code;
    entry.message = record.message.chars().take(1000).collect();
    for name in ["stdout.log", "stderr.log", "steam-0.log"] {
        let path = logs.join(name);
        if fs::symlink_metadata(&path).is_err_and(|e|e.kind() == std::io::ErrorKind::NotFound) { entry.partial_logs |= name != "steam-0.log"; continue; }
        let report = diagnostics::analyze_exact_log(&path, &record.id, "");
        entry.partial_logs |= report.truncated || report.status == "unreadable" || (name != "steam-0.log" && fs::metadata(&path).is_ok_and(|m|m.len() >= 2*1024*1024));
        for finding in report.findings {
            if !entry.summaries.contains(&finding.title) { entry.summaries.push(finding.title); }
        }
    }
    entry.partial_logs |= record.message.contains("output capture failed");
    entry.summaries.truncate(20); write(&logs, &entry)
}
fn observed_state(entry: &mut Entry, record: &Record, active: bool, boot: Option<&str>) {
    if entry.state != "running" { return; }
    if entry.boot_id.as_deref().zip(boot).is_some_and(|(old, current)|old != current) {
        entry.state = "interrupted".into(); entry.message = "System restarted before an exit result was saved.".into();
    } else if record.run_id.as_deref() != Some(&entry.id) || !active {
        entry.state = "unknown".into(); entry.message = "No exit result was saved. This does not confirm the game has stopped.".into();
    }
}
pub(super) fn list_in(dir: &Path, record: &Record, active: bool) -> Result<History, String> {
    let root = dir.join("logs");
    if fs::symlink_metadata(&root).is_err_and(|e|e.kind() == std::io::ErrorKind::NotFound) { return Ok(History {entries:vec![],warnings:vec![]}); }
    directory(&root, false)?;
    // Run IDs contain a process ID followed by nanoseconds. Sort by time, not PID.
    let mut newest = BinaryHeap::new();
    for item in fs::read_dir(&root).map_err(|e|e.to_string())? {
        let item = item.map_err(|e|e.to_string())?;
        let id = item.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) { continue; }
        let Some(time) = id.rsplit('-').next().and_then(|s|s.parse::<u128>().ok()) else {continue};
        if fs::symlink_metadata(item.path().join("history.json")).is_err_and(|e|e.kind() == std::io::ErrorKind::NotFound) { continue; }
        newest.push(Reverse((time, id))); if newest.len() > LIMIT { newest.pop(); }
    }
    let mut ordered = newest.into_iter().map(|Reverse(value)|value).collect::<Vec<_>>(); ordered.sort_by(|a,b|b.cmp(a));
    let mut entries = vec![]; let mut unreadable = 0;
    let boot = boot_id();
    for (_, id) in ordered {
        let logs = root.join(&id);
        if fs::symlink_metadata(logs.join("history.json")).is_err_and(|e|e.kind() == std::io::ErrorKind::NotFound) { continue; }
        match read(&logs, &id) {
            Ok(mut entry) => { observed_state(&mut entry, record, active, boot.as_deref()); entries.push(entry); }
            Err(_) => unreadable += 1,
        }
    }
    Ok(History {entries, warnings: if unreadable > 0 {vec![format!("{unreadable} recent run record(s) could not be read. Existing logs are unchanged.")]} else {vec![]}})
}
pub(super) fn latest_game_launch(dir:&Path,run:Option<&str>)->Option<u64>{
    let id=run.filter(|id|valid_id(id))?;let root=dir.join("logs");directory(&root,false).ok()?;
    let entry=read(&root.join(id),id).ok()?;
    (entry.kind=="game" && entry.state!="start-failed" && entry.started_at<=now().ok()?).then_some(entry.started_at)
}
pub fn list(id: String) -> Result<History, String> {
    let (dir, record) = load(&root()?, &id)?;
    let active = JOBS.lock().map_err(|_|"Process registry unavailable")?.contains(&id) || lock_profile(&dir).is_err();
    list_in(&dir, &record, active)
}

#[cfg(test)] mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self { let path=std::env::temp_dir().join(format!("pm-history-{}",stamp().unwrap())); fs::create_dir(&path).unwrap(); fs::create_dir(path.join("logs")).unwrap(); Self(path) }
        fn record(&self, id:&str) -> Record { serde_json::from_value(serde_json::json!({"id":"123","name":"Game","runtimeId":"wine","installer":"","gameExe":null,"state":"game-running","exitCode":null,"message":"Running","runId":id,"bootId":boot_id()})).unwrap() }
        fn begin(&self,id:&str) -> Record {
            let record=self.record(id); let logs=self.0.join("logs").join(id);fs::create_dir(&logs).unwrap();
            let runtime=Runtime{proton:None,id:"wine".into(),name:"Original Wine".into(),kind:"wine".into(),binary:PathBuf::from("/unused"),steam:None};
            begin(&logs,&record,&runtime,false).unwrap();record
        }
    }
    impl Drop for Fixture {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    #[test] fn recent_launch_fallback_excludes_installers_and_failed_starts(){
        let f=Fixture::new();let mut r=f.begin("1-100");
        assert!(latest_game_launch(&f.0,Some("1-100")).is_some());
        r.state="start-failed".into();finish(&f.0,&r).unwrap();assert_eq!(latest_game_launch(&f.0,Some("1-100")),None);
        let logs=f.0.join("logs/1-100");let mut entry=read(&logs,"1-100").unwrap();entry.kind="installer".into();entry.state="installer-exited".into();write(&logs,&entry).unwrap();
        assert_eq!(latest_game_launch(&f.0,Some("1-100")),None);assert_eq!(latest_game_launch(&f.0,Some("../1")),None);
    }
    #[test] fn latest_twenty_are_sorted_by_timestamp_and_older_logs_are_preserved() {
        let f=Fixture::new();
        for time in 1..=25 {f.begin(&format!("{}-{time}",100-time));}
        let history=list_in(&f.0,&f.record("75-25"),true).unwrap();
        assert_eq!(history.entries.len(),20);assert_eq!(history.entries[0].id,"75-25");assert_eq!(history.entries[19].id,"94-6");
        assert_eq!(fs::read_dir(f.0.join("logs")).unwrap().count(),25);
    }
    #[test] fn empty_run_directories_do_not_displace_saved_receipts() {
        let f=Fixture::new();
        for time in 1..=20 { f.begin(&format!("1-{time}")); }
        for time in 21..=40 { fs::create_dir(f.0.join(format!("logs/1-{time}"))).unwrap(); }
        let record=f.record("1-20");
        let history=list_in(&f.0,&record,false).unwrap();
        assert_eq!(history.entries.len(),20);assert!(history.warnings.is_empty());
        assert_eq!(fs::read_dir(f.0.join("logs")).unwrap().count(),40);
        let malformed=f.0.join("logs/1-40/history.json");fs::write(&malformed,"broken").unwrap();
        let history=list_in(&f.0,&record,false).unwrap();
        assert_eq!(history.entries.len(),19);assert_eq!(history.warnings.len(),1);
        assert_eq!(fs::read_to_string(malformed).unwrap(),"broken");
    }
    #[test] fn completion_keeps_original_runtime_and_caches_error_summaries() {
        let f=Fixture::new();let mut record=f.begin("1-100");let logs=f.0.join("logs/1-100");
        fs::write(logs.join("stderr.log"),"wine: Unhandled exception fixture").unwrap();fs::write(logs.join("stdout.log"),"").unwrap();
        record.runtime_id="changed-later".into();record.state="process-failed".into();record.exit_code=Some(7);record.message="Game process ended.".into();
        finish(&f.0,&record).unwrap();let first=read(&logs,"1-100").unwrap();
        assert_eq!(first.runtime,"Original Wine");assert_eq!(first.exit_code,Some(7));assert!(first.ended_at.is_some());assert!(!first.summaries.is_empty());
        fs::write(logs.join("stderr.log"),"").unwrap();finish(&f.0,&record).unwrap();
        let second=read(&logs,"1-100").unwrap();assert_eq!(first.summaries,second.summaries);assert_eq!(first.ended_at,second.ended_at);
    }
    #[test] fn old_runs_without_metadata_are_not_given_invented_timestamps() {
        let f=Fixture::new();fs::create_dir(f.0.join("logs/1-100")).unwrap();fs::write(f.0.join("logs/1-100/stderr.log"),"existing").unwrap();
        let r=f.record("1-100");finish(&f.0,&r).unwrap();assert!(list_in(&f.0,&r,false).unwrap().entries.is_empty());
        assert_eq!(fs::read_to_string(f.0.join("logs/1-100/stderr.log")).unwrap(),"existing");
    }
    #[test] fn unreadable_and_symlinked_history_are_reported_without_repair() {
        let f=Fixture::new();let r=f.begin("1-100");let file=f.0.join("logs/1-100/history.json");fs::write(&file,"broken").unwrap();
        assert_eq!(list_in(&f.0,&r,false).unwrap().warnings.len(),1);assert_eq!(fs::read_to_string(&file).unwrap(),"broken");
        fs::remove_file(&file).unwrap();std::os::unix::fs::symlink("/etc/passwd",&file).unwrap();assert!(finish(&f.0,&r).is_err());assert_eq!(list_in(&f.0,&r,false).unwrap().warnings.len(),1);
    }
    #[test] fn identity_mismatch_and_oversized_metadata_are_rejected() {
        let f=Fixture::new();let r=f.begin("1-100");let logs=f.0.join("logs/1-100");let mut entry=read(&logs,"1-100").unwrap();entry.id="2-200".into();write(&logs,&entry).unwrap();
        assert!(list_in(&f.0,&r,false).unwrap().entries.is_empty());
        fs::write(logs.join("history.json"),vec![b'x';65537]).unwrap();assert!(read(&logs,"1-100").is_err());
    }
    #[test] fn unresolved_runs_are_never_presented_as_successful_exits() {
        let f=Fixture::new();let r=f.begin("1-100");let original=read(&f.0.join("logs/1-100"),"1-100").unwrap();
        let mut entry=original.clone();entry.boot_id=Some("old".into());observed_state(&mut entry,&r,false,Some("new"));assert_eq!(entry.state,"interrupted");assert_eq!(entry.exit_code,None);assert_eq!(entry.ended_at,None);
        let mut entry=original.clone();let boot=entry.boot_id.clone();observed_state(&mut entry,&r,false,boot.as_deref());assert_eq!(entry.state,"unknown");
        let mut entry=original;let boot=entry.boot_id.clone();observed_state(&mut entry,&r,true,boot.as_deref());assert_eq!(entry.state,"running");
    }
}
