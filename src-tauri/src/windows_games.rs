//! User-approved Windows installers in dedicated prefixes. Never executes a shell.
use crate::{diagnostics, scanner};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, fs::{self, File, OpenOptions}, io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt}, path::{Path,PathBuf},
    process::{Command,Stdio}, sync::{LazyLock,Mutex}, time::{SystemTime,UNIX_EPOCH}};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Runtime { pub id:String, pub name:String, pub kind:String, #[serde(skip)] binary:PathBuf, #[serde(skip)] steam:Option<PathBuf> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
struct Record { id:String, name:String, runtime_id:String, installer:String, game_exe:Option<String>, state:String, exit_code:Option<i32>, message:String, run_id:Option<String>, #[serde(default)] boot_id:Option<String> }
#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Game { pub id:String, pub name:String, pub runtime_id:String, pub game_exe:Option<String>, pub state:String, pub exit_code:Option<i32>, pub message:String, pub directory:String, pub windows_drive:String, pub log_directory:Option<String> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Library { pub runtimes:Vec<Runtime>, pub games:Vec<Game>, pub warnings:Vec<String> }
static JOBS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(||Mutex::new(HashSet::new()));
static UNSAVED: LazyLock<Mutex<HashMap<String, Record>>> = LazyLock::new(||Mutex::new(HashMap::new()));
fn boot_id()->Option<String>{fs::read_to_string("/proc/sys/kernel/random/boot_id").ok().map(|s|s.trim().to_owned()).filter(|s|!s.is_empty())}
fn require_idle(r:&Record)->Result<(),String>{
    if r.state.ends_with("running") && !(r.boot_id.is_some() && boot_id().is_some() && r.boot_id!=boot_id()) {
        return Err("Previous process status is unresolved. Restart Linux before using this profile again; no process will be killed automatically.".into());
    } Ok(())
}
fn persist_completion(dir:&Path, record:&Record)->Result<(),String>{
    match save(dir,record){Ok(())=>Ok(()),Err(e)=>{
        UNSAVED.lock().map_err(|_|"Completion receipt lock unavailable")?.insert(record.id.clone(),record.clone());
        Err(e)
    }}
}
fn stamp()->Result<String,String>{Ok(format!("{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos()))}
fn valid_id(id:&str)->bool{!id.is_empty() && id.len()<80 && id.bytes().all(|b|b.is_ascii_digit() || b==b'-')}
fn regular(path:&Path,max:u64)->Result<(),String>{let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;if !m.is_file() || m.file_type().is_symlink() || m.len()>max{return Err("Expected a bounded regular file, not a symlink".into());}Ok(())}
fn directory(path:&Path,create:bool)->Result<(),String>{
    if create && !path.exists(){let mut b=fs::DirBuilder::new();b.mode(0o700);match b.create(path){Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(e)=>return Err(e.to_string())}}
    let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;if !m.is_dir() || m.file_type().is_symlink(){return Err("Managed game directories must not be symbolic links".into());}Ok(())
}
fn root()->Result<PathBuf,String>{
    let base=std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(||std::env::var_os("HOME").map(|h|PathBuf::from(h).join(".local/share"))).ok_or("Application data directory unavailable")?;
    if !base.is_absolute(){return Err("Application data directory must be absolute".into());}
    fs::create_dir_all(&base).map_err(|e|e.to_string())?;let base=fs::canonicalize(base).map_err(|e|e.to_string())?;
    let app=base.join("proton-medic");directory(&app,true)?;let root=app.join("windows-games");directory(&root,true)?;Ok(root)
}
fn profile_dir(root:&Path,id:&str)->Result<PathBuf,String>{if !valid_id(id){return Err("Invalid managed game ID".into());}let p=root.join(id);directory(&p,false)?;Ok(p)}
fn save(dir:&Path,r:&Record)->Result<(),String>{
    directory(dir,false)?;let target=dir.join("profile.json");if target.exists(){regular(&target,65536)?;}
    let tmp=dir.join(format!(".profile-{}.tmp",stamp()?));let bytes=serde_json::to_vec_pretty(r).map_err(|e|e.to_string())?;
    let result=(||{let mut f=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e|e.to_string())?;f.write_all(&bytes).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;fs::rename(&tmp,&target).map_err(|e|e.to_string())?;File::open(dir).and_then(|f|f.sync_all()).map_err(|e|e.to_string())})();
    if result.is_err(){let _=fs::remove_file(tmp);}result
}
fn load(root:&Path,id:&str)->Result<(PathBuf,Record),String>{let dir=profile_dir(root,id)?;let file=dir.join("profile.json");regular(&file,65536)?;let record:Record=serde_json::from_slice(&fs::read(file).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;if record.id!=id{return Err("Managed game identity mismatch".into());}Ok((dir,record))}
fn view(dir:&Path,r:Record,active:bool)->Game{
    let orphan=r.state.ends_with("running") && !active;let recovered=orphan && require_idle(&r).is_ok();
    Game{id:r.id,name:r.name,runtime_id:r.runtime_id,game_exe:r.game_exe,state:if recovered{"interrupted".into()}else if orphan{"session-ended".into()}else{r.state},exit_code:r.exit_code,
        message:if recovered{"Previous run was interrupted by a system restart. You can select or launch the game again.".into()}else if orphan{"Previous process status is unknown. Restart Linux before reusing this profile. No game data will be deleted.".into()}else{r.message},directory:dir.to_string_lossy().into(),windows_drive:dir.join("compatdata/pfx/drive_c").to_string_lossy().into(),
        log_directory:r.run_id.filter(|id|valid_id(id)).map(|id|dir.join("logs").join(id).to_string_lossy().into())}
}
fn executable(path:&Path)->Option<PathBuf>{let p=fs::canonicalize(path).ok()?;let m=fs::metadata(&p).ok()?;if m.is_file() && m.permissions().mode()&0o111!=0{Some(p)}else{None}}
pub fn runtimes()->Vec<Runtime>{
    let mut out=vec![];let mut seen=HashSet::new();
    if let Some(path)=std::env::var_os("PATH") {for name in ["wine","wine64"] {if let Some(p)=std::env::split_paths(&path).filter(|p|p.is_absolute()).find_map(|dir|executable(&dir.join(name))){if seen.insert(p.clone()){out.push(Runtime{id:format!("wine:{}",p.display()),name:format!("System {name}"),kind:"wine".into(),binary:p,steam:None});}}}}
    if let Some(p)=executable(Path::new("/usr/lib/wine/wine64")){if seen.insert(p.clone()){out.push(Runtime{id:format!("wine:{}",p.display()),name:"System Wine64".into(),kind:"wine".into(),binary:p,steam:None});}}
    let scan=scanner::scan();
    for tool in &scan.compatibility_tools {
        let path=Path::new(&tool.path);
        // Flatpak runtimes must run inside their container; do not execute them on the host.
        if path.components().any(|c|c.as_os_str()==".var"){continue;}
        let Some(binary)=executable(&path.join("proton")) else{continue};
        let Some(steam)=scan.installations.iter().find(|i|!i.kind.eq_ignore_ascii_case("flatpak")) else{continue};
        if seen.insert(binary.clone()){out.push(Runtime{id:format!("proton:{}",binary.display()),name:tool.name.clone(),kind:"proton".into(),binary,steam:Some(PathBuf::from(&steam.path))});}
    }
    out
}
fn runtime(id:&str)->Result<Runtime,String>{runtimes().into_iter().find(|r|r.id==id).ok_or_else(||"Selected Wine/Proton runtime is not available; refresh the runtime list".into())}
fn windows_exe(path:&Path)->Result<PathBuf,String>{
    if !path.is_absolute() || !path.extension().is_some_and(|x|x.eq_ignore_ascii_case("exe")){return Err("Choose an absolute path to a Windows .exe file".into());}
    let path=fs::canonicalize(path).map_err(|e|e.to_string())?;regular(&path,u64::MAX)?;
    let mut f=File::open(&path).map_err(|e|e.to_string())?;let mut header=[0u8;64];f.read_exact(&mut header).map_err(|_|"Installer is too short to be a Windows executable")?;
    if &header[..2]!=b"MZ"{return Err("File is not a Windows executable (missing MZ header)".into());}
    let offset=u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
    if offset<64 || offset.checked_add(6).is_none_or(|n|n>f.metadata().map(|m|m.len()).unwrap_or(0)){return Err("Invalid Windows PE header offset".into());}
    f.seek(SeekFrom::Start(offset)).map_err(|e|e.to_string())?;let mut pe=[0u8;6];f.read_exact(&mut pe).map_err(|e|e.to_string())?;
    if &pe[..4]!=b"PE\0\0" || !matches!(u16::from_le_bytes([pe[4],pe[5]]),0x14c|0x8664){return Err("Only x86/x64 Windows PE executables are supported".into());}Ok(path)
}
fn managed_exe(dir:&Path,path:&Path)->Result<PathBuf,String>{
    let drive=dir.join("compatdata/pfx/drive_c");for p in [dir.join("compatdata"),dir.join("compatdata/pfx"),drive.clone()]{directory(&p,false)?;}
    let drive=fs::canonicalize(drive).map_err(|e|e.to_string())?;let exe=windows_exe(path)?;
    if !exe.starts_with(&drive){return Err("Choose the installed game executable inside this managed game's Windows C: drive".into());}Ok(exe)
}
fn command(r:&Runtime,dir:&Path,exe:&Path,logs:&Path)->Command{
    let mut cmd=crate::font_runtime::child_command(&r.binary);if r.kind=="proton"{cmd.arg("run");}
    cmd.arg(exe).current_dir(exe.parent().unwrap_or(dir)).env("WINEPREFIX",dir.join("compatdata/pfx"))
        .env("STEAM_COMPAT_DATA_PATH",dir.join("compatdata")).env("PROTON_LOG","1").env("PROTON_LOG_DIR",logs)
        .env_remove("WINEARCH").env("WINEDEBUG","-all,+err,+warn").env("SteamAppId","0").env("SteamGameId","0")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(steam)=&r.steam{cmd.env("STEAM_COMPAT_CLIENT_INSTALL_PATH",steam);}cmd
}
fn capture(mut input:impl Read,mut output:File)->std::io::Result<()> {
    const LIMIT:usize=2*1024*1024;
    let mut tail=std::collections::VecDeque::with_capacity(LIMIT);let mut buf=[0u8;8192];let mut since_flush=0;
    loop {let n=input.read(&mut buf)?;if n==0{output.seek(SeekFrom::Start(0))?;let(a,b)=tail.as_slices();output.write_all(a)?;output.write_all(b)?;output.set_len(tail.len() as u64)?;break;}
        let discard=(tail.len()+n).saturating_sub(LIMIT);tail.drain(..discard);tail.extend(&buf[..n]);
        since_flush+=n;if since_flush<256*1024 {continue;}since_flush=0;
        output.seek(SeekFrom::Start(0))?;let (a,b)=tail.as_slices();output.write_all(a)?;output.write_all(b)?;output.set_len(tail.len() as u64)?;
    } output.sync_all()
}
fn lock_profile(dir:&Path)->Result<File,String>{
    let file=OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(dir.join("profile.lock")).map_err(|e|e.to_string())?;
    file.try_lock().map_err(|_|"Another Proton Medic instance is changing this profile")?;Ok(file)
}
fn spawn(dir:PathBuf,mut record:Record,r:Runtime,exe:PathBuf,install:bool)->Result<Game,String>{
    let mut jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;
    if jobs.contains(&record.id){return Err("This game's installer or game is already running in this app session".into());}
    let _profile_lock=lock_profile(&dir)?;
    let (_,fresh)=load(dir.parent().ok_or("Profile root unavailable")?,&record.id)?;require_idle(&fresh)?;
    if jobs.len()>=4{return Err("Close an installer/game before starting more (four active jobs maximum)".into());}
    let run=stamp()?;let log_root=dir.join("logs");directory(&log_root,true)?;let logs=log_root.join(&run);directory(&logs,true)?;
    for p in [dir.join("compatdata"),dir.join("compatdata/pfx")]{directory(&p,true)?;}
    let stdout=OpenOptions::new().write(true).create_new(true).mode(0o600).open(logs.join("stdout.log")).map_err(|e|e.to_string())?;
    let stderr=OpenOptions::new().write(true).create_new(true).mode(0o600).open(logs.join("stderr.log")).map_err(|e|e.to_string())?;
    record.boot_id=boot_id();record.run_id=Some(run);record.exit_code=None;record.state=if install{"installer-running"}else{"game-running"}.into();
    record.message=if install{"Installer started. Complete its Windows setup wizard, then select the installed game executable."}else{"Game process started; confirm gameplay yourself."}.into();save(&dir,&record)?;
    let child=command(&r,&dir,&exe,&logs).spawn();
    let mut child=match child{Ok(c)=>c,Err(e)=>{record.state="start-failed".into();record.message=format!("Runtime failed to start: {e}");save(&dir,&record)?;return Ok(view(&dir,record,false));}};
    let out=child.stdout.take().ok_or("Runtime stdout unavailable")?;let err=child.stderr.take().ok_or("Runtime stderr unavailable")?;
    jobs.insert(record.id.clone());let result=view(&dir,record.clone(),true);
    std::thread::spawn(move||{
        let a=std::thread::spawn(move||capture(out,stdout));let b=std::thread::spawn(move||capture(err,stderr));
        let status=child.wait();let capture_ok=[a.join(),b.join()].into_iter().all(|r|matches!(r,Ok(Ok(()))));
        record.exit_code=status.as_ref().ok().and_then(|s|s.code());
        record.state=if status.as_ref().is_ok_and(|s|s.success()){if install{"installer-exited"}else{"game-exited"}}else{"process-failed"}.into();
        record.message=if install{"Installer process ended. This is not installation proof: select and test the installed game executable."}else{"Game process ended. Exit status does not establish playability."}.into();
        if let Err(e)=status{record.message=format!("Could not observe runtime exit: {e}");}
        if !capture_ok{record.message.push_str(" Runtime output capture failed; logs may be incomplete.");}
        if let Ok(mut jobs)=JOBS.lock(){if persist_completion(&dir,&record).is_ok(){jobs.remove(&record.id);}}
    });Ok(result)
}
pub fn list()->Result<Library,String>{
    let root=root()?;let mut jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;let mut games=vec![];let mut warnings=vec![];
    let mut pending=UNSAVED.lock().map_err(|_|"Completion receipt lock unavailable")?;
    pending.retain(|id,record|{let dir=root.join(id);match save(&dir,record){Ok(())=>{jobs.remove(id);false},Err(e)=>{warnings.push(format!("Completion for {} could not be saved: {e}. Keep Proton Medic open; it will retry.",record.name));true}}});
    for item in fs::read_dir(&root).map_err(|e|e.to_string())?.flatten(){let id=item.file_name().to_string_lossy().into_owned();if !valid_id(&id){continue;}match load(&root,&id){Ok((dir,mut r))=>{
            if r.state.ends_with("running") && r.boot_id.is_none(){
                let migrate=(||{let _lock=lock_profile(&dir)?;let(_,mut latest)=load(&root,&id)?;
                    if latest.state.ends_with("running") && latest.boot_id.is_none(){latest.boot_id=boot_id();save(&dir,&latest)?;}Ok::<_,String>(latest)})();
                match migrate{Ok(latest)=>r=latest,Err(e)=>warnings.push(format!("Could not record recovery state for {id}: {e}"))}
            }
            games.push(view(&dir,r,jobs.contains(&id)));
        },Err(e)=>warnings.push(format!("Could not read managed game {id}: {e}"))}}
    games.sort_by(|a,b|a.name.cmp(&b.name));Ok(Library{runtimes:runtimes(),games,warnings})
}
pub fn install(name:String,installer:String,runtime_id:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve running the selected Windows installer".into());}
    if name.trim().is_empty() || name.len()>120 || name.chars().any(char::is_control){return Err("Enter a game name of 1–120 characters".into());}
    let exe=windows_exe(Path::new(&installer))?;let runtime=runtime(&runtime_id)?;let root=root()?;
    let id=stamp()?;let dir=root.join(&id);directory(&dir,true)?;
    let record=Record{id,name:name.trim().into(),runtime_id,installer:exe.to_string_lossy().into(),game_exe:None,state:"created".into(),exit_code:None,message:String::new(),run_id:None,boot_id:None};save(&dir,&record)?;
    spawn(dir,record,runtime,exe,true)
}
pub fn select_executable(id:String,path:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve the installed executable selection".into());}
    let root=root()?;let jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;if jobs.contains(&id){return Err("Wait for this installer/game process to finish first".into());}
    let dir=profile_dir(&root,&id)?;let _profile_lock=lock_profile(&dir)?;let(dir,mut r)=load(&root,&id)?;require_idle(&r)?;let exe=managed_exe(&dir,Path::new(&path))?;
    r.game_exe=Some(exe.to_string_lossy().into());r.state="ready-to-test".into();r.message="Executable selected. Launch and confirm gameplay yourself.".into();save(&dir,&r)?;Ok(view(&dir,r,false))
}
pub fn launch(id:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve opening this Windows game".into());}
    let(dir,r)=load(&root()?,&id)?;let exe=managed_exe(&dir,Path::new(r.game_exe.as_deref().ok_or("Select the installed game's executable first")?))?;let runtime=runtime(&r.runtime_id)?;spawn(dir,r,runtime,exe,false)
}
#[derive(Serialize)]
pub struct LogReport { findings:Vec<diagnostics::Finding>, logs:Vec<LogStatus> }
#[derive(Serialize)]
pub struct LogStatus { name:String, status:String, truncated:bool }
pub fn findings(id:String)->Result<LogReport,String>{
    let(dir,r)=load(&root()?,&id)?;let run=r.run_id.filter(|id|valid_id(id)).ok_or("No runtime log recorded yet")?;let root=dir.join("logs");directory(&root,false)?;let logs=root.join(run);directory(&logs,false)?;
    let mut by_code=HashMap::new();let mut statuses=vec![];for name in ["stdout.log","stderr.log","steam-0.log"]{let p=logs.join(name);let detail=diagnostics::analyze_exact_log(&p,&id,&dir.to_string_lossy());
        statuses.push(LogStatus{name:name.into(),status:if fs::symlink_metadata(&p).is_err_and(|e|e.kind()==std::io::ErrorKind::NotFound){"missing".into()}else{detail.status.clone()},truncated:detail.truncated || (name!="steam-0.log" && fs::metadata(&p).is_ok_and(|m|m.len()>=2*1024*1024))});
        for finding in detail.findings{by_code.entry(finding.code.clone()).or_insert(finding);}
    }
    let mut out:Vec<_>=by_code.into_values().collect();out.sort_by(|a,b|a.code.cmp(&b.code));Ok(LogReport{findings:out,logs:statuses})
}

#[cfg(test)]
mod tests{
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture{fn new()->Self{let p=std::env::temp_dir().join(format!("pm-exe-{}",stamp().unwrap()));fs::create_dir(&p).unwrap();Self(p)}}
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    fn pe(path:&Path){let mut bytes=vec![0u8;256];bytes[..2].copy_from_slice(b"MZ");bytes[60..64].copy_from_slice(&128u32.to_le_bytes());bytes[128..132].copy_from_slice(b"PE\0\0");bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());fs::write(path,bytes).unwrap();}
    fn record(id:&str)->Record{Record{id:id.into(),name:"Fixture".into(),runtime_id:"fixture".into(),installer:String::new(),game_exe:None,state:"created".into(),exit_code:None,message:String::new(),run_id:None,boot_id:None}}
    #[test] fn validates_windows_pe_and_rejects_linux_and_truncated_files(){
        let f=Fixture::new();let exe=f.0.join("setup.exe");pe(&exe);assert!(windows_exe(&exe).is_ok());
        fs::write(&exe,b"#!/bin/sh\necho not windows").unwrap();assert!(windows_exe(&exe).is_err());
        assert!(windows_exe(Path::new("relative.exe")).is_err());
        pe(&exe);let bad=f.0.join("installer.bin");fs::copy(&exe,&bad).unwrap();assert!(windows_exe(&bad).is_err());
    }
    #[test] fn rejects_invalid_headers_and_unsupported_architecture(){
        let f=Fixture::new();let exe=f.0.join("bad.exe");pe(&exe);let mut bytes=fs::read(&exe).unwrap();bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());fs::write(&exe,bytes).unwrap();assert!(windows_exe(&exe).is_err());
        pe(&exe);let mut bytes=fs::read(&exe).unwrap();bytes[132..134].copy_from_slice(&0xaa64u16.to_le_bytes());fs::write(&exe,bytes).unwrap();assert!(windows_exe(&exe).is_err());
    }
    #[test] fn game_selection_cannot_escape_managed_drive(){
        let f=Fixture::new();fs::create_dir_all(f.0.join("compatdata/pfx/drive_c/Game")).unwrap();let outside=f.0.join("outside.exe");pe(&outside);assert!(managed_exe(&f.0,&outside).is_err());
        let inside=f.0.join("compatdata/pfx/drive_c/Game/game.exe");pe(&inside);assert!(managed_exe(&f.0,&inside).is_ok());
        let link=f.0.join("compatdata/pfx/drive_c/escape.exe");std::os::unix::fs::symlink(&outside,&link).unwrap();assert!(managed_exe(&f.0,&link).is_err());
    }
    #[test] fn persisted_profiles_survive_reload_and_reject_identity_or_symlink_changes(){
        let f=Fixture::new();let id=stamp().unwrap();let dir=f.0.join(&id);directory(&dir,true).unwrap();save(&dir,&record(&id)).unwrap();assert_eq!(load(&f.0,&id).unwrap().1.name,"Fixture");
        assert!(load(&f.0,"../../etc").is_err());save(&dir,&record("999")).unwrap();assert!(load(&f.0,&id).is_err());
        fs::remove_file(dir.join("profile.json")).unwrap();std::os::unix::fs::symlink("/etc/passwd",dir.join("profile.json")).unwrap();assert!(load(&f.0,&id).is_err());
    }
    #[test] fn explicit_consent_is_required_before_any_install_or_launch_io(){
        assert!(install("Fixture".into(),"/none".into(),"none".into(),false).unwrap_err().contains("Approve"));
        assert!(launch("none".into(),false).unwrap_err().contains("Approve"));
        assert!(select_executable("none".into(),"/none".into(),false).unwrap_err().contains("Approve"));
    }
    #[test] fn command_preserves_spaces_and_uses_explicit_profile_environment(){
        let r=Runtime{id:"fixture".into(),name:"Fixture".into(),kind:"proton".into(),binary:PathBuf::from("/fixture/proton"),steam:Some(PathBuf::from("/fixture/Steam"))};
        let cmd=command(&r,Path::new("/managed/123"),Path::new("/Downloads/Game Setup.exe"),Path::new("/managed/123/logs/1"));
        assert_eq!(cmd.get_args().collect::<Vec<_>>(),vec![std::ffi::OsStr::new("run"),std::ffi::OsStr::new("/Downloads/Game Setup.exe")]);
        let vars:HashMap<_,_>=cmd.get_envs().collect();assert_eq!(vars[std::ffi::OsStr::new("WINEPREFIX")],Some(std::ffi::OsStr::new("/managed/123/compatdata/pfx")));
        assert_eq!(vars[std::ffi::OsStr::new("PROTON_LOG")],Some(std::ffi::OsStr::new("1")));
    }
    #[test] fn output_capture_is_bounded(){let f=Fixture::new();let file=f.0.join("log");capture(std::io::Cursor::new(vec![b'x';3*1024*1024]),File::create(&file).unwrap()).unwrap();assert_eq!(fs::metadata(file).unwrap().len(),2*1024*1024);}
    #[test] fn fake_runtime_executes_installer_protocol_and_persists_exit(){
        let f=Fixture::new();let id=stamp().unwrap();let dir=f.0.join(&id);directory(&dir,true).unwrap();let exe=f.0.join("Setup With Spaces.exe");pe(&exe);
        let fake=f.0.join("fake-wine");fs::write(&fake,"#!/bin/sh\nmkdir -p \"$WINEPREFIX/drive_c/Game\"\ncp \"$1\" \"$WINEPREFIX/drive_c/Game/game.exe\"\nprintf 'unhandled exception fixture\\n' >&2\nexit 7\n").unwrap();fs::set_permissions(&fake,fs::Permissions::from_mode(0o700)).unwrap();
        let r=Runtime{id:"fixture".into(),name:"Fake Wine".into(),kind:"wine".into(),binary:fake,steam:None};let rec=record(&id);save(&dir,&rec).unwrap();
        let started=spawn(dir.clone(),rec,r,exe,true).unwrap();assert_eq!(started.state,"installer-running");
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
        while JOBS.lock().unwrap().contains(&id){assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(10));}
        let(_,done)=load(&f.0,&id).unwrap();assert_eq!(done.exit_code,Some(7));assert_eq!(done.state,"process-failed");
        assert!(managed_exe(&dir,&dir.join("compatdata/pfx/drive_c/Game/game.exe")).is_ok());
        let log=dir.join("logs").join(done.run_id.unwrap()).join("stderr.log");assert!(fs::read_to_string(log).unwrap().contains("unhandled exception"));
    }
    #[test] fn missing_runtime_is_recorded_as_failure_without_claiming_installation(){
        let f=Fixture::new();let id=stamp().unwrap();let dir=f.0.join(&id);directory(&dir,true).unwrap();let exe=f.0.join("setup.exe");pe(&exe);
        let r=Runtime{id:"none".into(),name:"Missing".into(),kind:"wine".into(),binary:f.0.join("absent"),steam:None};
        save(&dir,&record(&id)).unwrap();let result=spawn(dir,record(&id),r,exe,true).unwrap();assert_eq!(result.state,"start-failed");assert!(result.game_exe.is_none());assert!(load(&f.0,&id).is_ok());
    }
    #[test] fn unknown_persisted_runs_are_blocked_until_a_proven_reboot(){
        let mut r=record("123");r.state="game-running".into();assert!(require_idle(&r).is_err());
        r.boot_id=boot_id();assert!(require_idle(&r).is_err());
        if boot_id().is_some(){r.boot_id=Some("previous-boot".into());assert!(require_idle(&r).is_ok());}
    }
    #[test] fn tail_capture_preserves_late_failure(){
        let f=Fixture::new();let p=f.0.join("tail");let mut data=vec![b'x';3*1024*1024];data.extend_from_slice(b"late crash marker");
        capture(std::io::Cursor::new(data),File::create(&p).unwrap()).unwrap();
        let bytes=fs::read(p).unwrap();assert_eq!(bytes.len(),2*1024*1024);assert!(bytes.ends_with(b"late crash marker"));
    }
    #[test] fn failed_completion_save_retains_receipt_for_retry(){
        let f=Fixture::new();let id=stamp().unwrap();let r=record(&id);let missing=f.0.join("missing");
        assert!(persist_completion(&missing,&r).is_err());assert!(UNSAVED.lock().unwrap().contains_key(&id));
        directory(&missing,true).unwrap();assert!(save(&missing,&r).is_ok());UNSAVED.lock().unwrap().remove(&id);
    }
    #[test] fn profile_lock_excludes_second_instance(){
        let f=Fixture::new();let guard=lock_profile(&f.0).unwrap();assert!(lock_profile(&f.0).is_err());drop(guard);assert!(lock_profile(&f.0).is_ok());
    }

}
