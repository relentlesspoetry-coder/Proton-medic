//! User-approved Windows installers in dedicated prefixes. Never executes a shell.
pub mod history;
pub mod monitor;
use crate::{diagnostics, scanner};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, fs::{self, File, OpenOptions}, io::{Read, Seek, SeekFrom, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt}, path::{Path,PathBuf},
    process::{Command,Stdio}, sync::{LazyLock,Mutex}, time::{SystemTime,UNIX_EPOCH}};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Runtime { pub id:String, pub name:String, pub kind:String, #[serde(skip)] binary:PathBuf, #[serde(skip)] steam:Option<PathBuf>, #[serde(skip)] proton:Option<PathBuf> }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
struct Record { id:String, name:String, runtime_id:String, installer:String, game_exe:Option<String>, state:String, exit_code:Option<i32>, message:String, run_id:Option<String>, #[serde(default)] boot_id:Option<String>, #[serde(default)] prefix_path:Option<String>, #[serde(default)] hidden:bool, #[serde(default)] launch_arguments:Vec<String>, #[serde(default)] working_directory:Option<String>, #[serde(default)] default_runtime_id:Option<String>, #[serde(default)] last_launched_at:Option<u64> }
#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Game { pub id:String, pub name:String, pub runtime_id:String, pub game_exe:Option<String>, pub state:String, pub exit_code:Option<i32>, pub message:String, pub directory:String, pub windows_drive:String, pub prefix_directory:String, pub log_directory:Option<String>, pub launch_arguments:Vec<String>, pub working_directory:Option<String>, pub last_launched_at:Option<u64> }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Library { pub runtimes:Vec<Runtime>, pub games:Vec<Game>, pub removed_games:Vec<Game>, pub warnings:Vec<String> }
static JOBS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(||Mutex::new(HashSet::new()));
static UNSAVED: LazyLock<Mutex<HashMap<String, Record>>> = LazyLock::new(||Mutex::new(HashMap::new()));
pub(crate) fn require_update_idle()->Result<(),String>{
    if !JOBS.try_lock().map_err(|_|"Wait for the current Windows game operation")?.is_empty() {
        return Err("Finish Windows games and installers before closing or updating. No process will be stopped automatically.".into());
    }
    if !UNSAVED.try_lock().map_err(|_|"Wait for the completion receipt")?.is_empty() {
        return Err("A game completion receipt is still being saved. Keep Proton Medic open and retry.".into());
    }
    Ok(())
}
fn boot_id()->Option<String>{fs::read_to_string("/proc/sys/kernel/random/boot_id").ok().map(|s|s.trim().to_owned()).filter(|s|!s.is_empty())}
fn require_idle(r:&Record)->Result<(),String>{
    if r.state.ends_with("running") && !(r.boot_id.is_some() && boot_id().is_some() && r.boot_id!=boot_id()) {
        return Err("Previous process status is unresolved. Restart Linux before using this profile again; no process will be killed automatically.".into());
    } Ok(())
}
fn persist_completion(dir:&Path, record:&Record)->Result<(),String>{
    match history::finish(dir,record).and_then(|_|save(dir,record)){Ok(())=>Ok(()),Err(e)=>{
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
    let base=std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p|p.is_absolute()).or_else(||std::env::var_os("HOME").map(|h|PathBuf::from(h).join(".local/share"))).ok_or("Application data directory unavailable")?;
    if !base.is_absolute(){return Err("Application data directory must be absolute".into());}
    fs::create_dir_all(&base).map_err(|e|e.to_string())?;let base=fs::canonicalize(base).map_err(|e|e.to_string())?;
    let app=base.join("proton-medic");directory(&app,true)?;let root=app.join("windows-games");directory(&root,true)?;Ok(root)
}
fn profile_dir(root:&Path,id:&str)->Result<PathBuf,String>{if !valid_id(id){return Err("Invalid managed game ID".into());}let p=root.join(id);directory(&p,false)?;Ok(p)}
fn save(dir:&Path,r:&Record)->Result<(),String>{
    directory(dir,false)?;let target=dir.join("profile.json");if target.exists(){regular(&target,65536)?;}
    let tmp=dir.join(format!(".profile-{}.tmp",stamp()?));let bytes=serde_json::to_vec_pretty(r).map_err(|e|e.to_string())?;
    if bytes.len()>65536{return Err("Profile exceeds 64 KiB. Shorten launch arguments or paths before saving.".into());}
    let result=(||{let mut f=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e|e.to_string())?;f.write_all(&bytes).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;fs::rename(&tmp,&target).map_err(|e|e.to_string())?;File::open(dir).and_then(|f|f.sync_all()).map_err(|e|e.to_string())})();
    if result.is_err(){let _=fs::remove_file(tmp);}result
}
fn load(root:&Path,id:&str)->Result<(PathBuf,Record),String>{let dir=profile_dir(root,id)?;let file=dir.join("profile.json");regular(&file,65536)?;let record:Record=serde_json::from_slice(&fs::read(file).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;if record.id!=id{return Err("Managed game identity mismatch".into());}Ok((dir,record))}
fn view(dir:&Path,r:Record,active:bool)->Game{
    let last_launched_at=r.last_launched_at.or_else(||history::latest_game_launch(dir,r.run_id.as_deref()));
    let prefix=prefix_path(dir,&r);
    let orphan=r.state.ends_with("running") && !active;let recovered=orphan && require_idle(&r).is_ok();
    Game{last_launched_at,id:r.id,name:r.name,runtime_id:r.runtime_id,game_exe:r.game_exe,state:if recovered{"interrupted".into()}else if orphan{"session-ended".into()}else{r.state},exit_code:r.exit_code,
        message:if recovered{"Previous run was interrupted by a system restart. You can select or launch the game again.".into()}else if orphan{"Previous process status is unknown. Restart Linux before reusing this profile. No game data will be deleted.".into()}else{r.message},directory:dir.to_string_lossy().into(),windows_drive:prefix.join("drive_c").to_string_lossy().into(),prefix_directory:prefix.to_string_lossy().into(),
        launch_arguments:r.launch_arguments,working_directory:r.working_directory,log_directory:r.run_id.filter(|id|valid_id(id)).map(|id|dir.join("logs").join(id).to_string_lossy().into())}
}
fn executable(path:&Path)->Option<PathBuf>{let p=fs::canonicalize(path).ok()?;let m=fs::metadata(&p).ok()?;if m.is_file() && m.permissions().mode()&0o111!=0{Some(p)}else{None}}
pub fn runtimes()->Vec<Runtime>{ discover_runtimes(&scanner::scan()) }
fn discover_runtimes(scan:&scanner::ScanReport)->Vec<Runtime>{
    let mut out=vec![];let mut seen=HashSet::new();
    if let Some(path)=std::env::var_os("PATH") {for name in ["wine","wine64"] {if let Some(p)=std::env::split_paths(&path).filter(|p|p.is_absolute()).find_map(|dir|executable(&dir.join(name))){if seen.insert(p.clone()){out.push(Runtime{proton:None,id:format!("wine:{}",p.display()),name:format!("System {name}"),kind:"wine".into(),binary:p,steam:None});}}}}
    if let Some(p)=executable(Path::new("/usr/lib/wine/wine64")){if seen.insert(p.clone()){out.push(Runtime{proton:None,id:format!("wine:{}",p.display()),name:"System Wine64".into(),kind:"wine".into(),binary:p,steam:None});}}
    for tool in &scan.compatibility_tools {
        let path=Path::new(&tool.path);
        // Flatpak runtimes must run inside their container; do not execute them on the host.
        if path.components().any(|c|c.as_os_str()==".var"){continue;}
        let Some(binary)=executable(&path.join("proton")) else{continue};
        if path.join(".proton-medic-managed").is_file() {
            if let Some(umu)=crate::proton_manager::umu(){
                if seen.insert(binary.clone()){out.push(Runtime{proton:Some(path.to_path_buf()),id:format!("proton:{}",binary.display()),name:format!("{} (UMU)",tool.name),kind:"proton".into(),binary:umu,steam:None});}
            }
            continue;
        }
        let Some(steam)=scan.installations.iter().find(|i|!i.kind.eq_ignore_ascii_case("flatpak")) else{continue};
        if seen.insert(binary.clone()){out.push(Runtime{proton:None,id:format!("proton:{}",binary.display()),name:tool.name.clone(),kind:"proton".into(),binary,steam:Some(PathBuf::from(&steam.path))});}
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
fn prefix_path(dir:&Path,record:&Record)->PathBuf {
    record.prefix_path.as_ref().map(PathBuf::from).unwrap_or_else(||dir.join("compatdata/pfx"))
}
fn validate_prefix(prefix:&Path)->Result<PathBuf,String>{
    if !prefix.is_absolute(){return Err("Choose an absolute Windows prefix path".into());}
    directory(prefix,false)?;directory(&prefix.join("drive_c"),false)?;
    regular(&prefix.join("system.reg"),64*1024*1024)?;
    fs::canonicalize(prefix).map_err(|e|e.to_string())
}
fn checked_exe(dir:&Path,record:&Record,path:&Path)->Result<PathBuf,String>{
    // A selected game can be on Z: or another drive; its prefix stays unchanged.
    let prefix=prefix_path(dir,record);directory(&prefix,false)?;
    directory(&prefix.join("drive_c"),false)?;windows_exe(path)
}
#[cfg(test)]
fn command(r:&Runtime,dir:&Path,exe:&Path,logs:&Path)->Command{
    command_with_prefix(r,dir,&dir.join("compatdata/pfx"),exe,logs)
}
fn command_with_prefix(r:&Runtime,dir:&Path,prefix:&Path,exe:&Path,_logs:&Path)->Command{
    use std::os::unix::process::CommandExt;
    let mut cmd=crate::font_runtime::child_command(&r.binary);if r.kind=="proton" && r.proton.is_none(){cmd.arg("run");}
    cmd.arg(exe).current_dir(exe.parent().unwrap_or(dir)).env("WINEPREFIX",prefix)
        .env("STEAM_COMPAT_DATA_PATH",prefix.parent().unwrap_or(dir)).env("PROTON_LOG","0").env_remove("PROTON_LOG_DIR")
        .env("DXVK_LOG_LEVEL","warn").env("DXVK_NVAPI_LOG_LEVEL","warn").env("VKD3D_DEBUG","warn").env("VKD3D_SHADER_DEBUG","warn")
        .env_remove("WINEARCH").env("WINEDEBUG","-all,+err,+warn").env("SteamAppId","0").env("SteamGameId","0")
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(proton)=&r.proton{cmd.env("PROTONPATH",proton).env("GAMEID","umu-default");}
    cmd.process_group(0);
    if let Some(steam)=&r.steam{cmd.env("STEAM_COMPAT_CLIENT_INSTALL_PATH",steam);}cmd
}
fn command_for_record(r:&Runtime,dir:&Path,prefix:&Path,exe:&Path,logs:&Path,record:&Record,install:bool)->Command{
    let mut command=command_with_prefix(r,dir,prefix,exe,logs);
    if !install {
        command.args(&record.launch_arguments);
        if let Some(working)=&record.working_directory {command.current_dir(working);}
    }
    command
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(rename_all="camelCase")]
pub struct LaunchSettings { pub runtime_id:String, pub arguments:Vec<String>, pub working_directory:Option<String> }
fn settings(r:&Record)->LaunchSettings {LaunchSettings{runtime_id:r.runtime_id.clone(),arguments:r.launch_arguments.clone(),working_directory:r.working_directory.clone()}}
fn validate_launch_settings(mut value:LaunchSettings)->Result<LaunchSettings,String>{
    if value.arguments.len()>64 || value.arguments.iter().any(|a|a.len()>4096 || a.chars().any(char::is_control)) || value.arguments.iter().map(String::len).sum::<usize>()>16384 {
        return Err("Use at most 64 arguments, 4096 bytes per argument and 16 KiB total, without control characters. Arguments are passed directly, without a shell.".into());
    }
    if let Some(path)=&value.working_directory {
        let path=Path::new(path);if !path.is_absolute(){return Err("Choose an absolute working directory".into());}
        directory(path,false)?;value.working_directory=Some(fs::canonicalize(path).map_err(|e|e.to_string())?.to_string_lossy().into());
    }
    Ok(value)
}
fn save_settings_in(root:&Path,id:&str,next:Option<LaunchSettings>,expected:LaunchSettings,resolve:impl FnOnce(&str)->Result<Runtime,String>)->Result<Game,String>{
    let dir=profile_dir(root,id)?;let _lock=lock_profile(&dir)?;let(_,mut r)=load(root,id)?;require_idle(&r)?;
    if settings(&r)!=expected {return Err("Launch settings changed in another window. Cancel editing, refresh, and review the latest values.".into());}
    let value=validate_launch_settings(next.unwrap_or_else(||LaunchSettings{runtime_id:r.default_runtime_id.clone().unwrap_or_else(||r.runtime_id.clone()),arguments:vec![],working_directory:None}))?;
    let rt=resolve(&value.runtime_id)?;
    if rt.kind=="proton" && prefix_path(&dir,&r).file_name().is_none_or(|n|n!="pfx") {return Err("Proton requires an existing prefix named pfx. Choose Wine for a Wine-style prefix; the prefix will not be moved.".into());}
    if settings(&r)!=value {
        backup_profile(&dir,&r)?;
        if r.default_runtime_id.is_none(){r.default_runtime_id=Some(r.runtime_id.clone());}
        r.runtime_id=value.runtime_id;r.launch_arguments=value.arguments;r.working_directory=value.working_directory;
        save(&dir,&r)?;
    }
    Ok(view(&dir,r,false))
}
pub fn save_launch_settings(id:String,value:LaunchSettings,expected:LaunchSettings,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve saving game launch settings".into());}
    save_settings_in(&root()?,&id,Some(value),expected,runtime)
}
pub fn reset_launch_settings(id:String,expected:LaunchSettings,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve resetting game launch settings".into());}
    save_settings_in(&root()?,&id,None,expected,runtime)
}
fn locked_record(dir:&Path,previous:&Record,runtime:&Runtime,exe:&Path,install:bool)->Result<Record,String>{
    let (_,fresh)=load(dir.parent().ok_or("Profile root unavailable")?,&previous.id)?;
    require_idle(&fresh)?;
    if fresh.runtime_id!=previous.runtime_id || fresh.prefix_path!=previous.prefix_path || fresh.game_exe!=previous.game_exe || fresh.installer!=previous.installer || fresh.launch_arguments!=previous.launch_arguments || fresh.working_directory!=previous.working_directory {
        return Err("Game launch settings changed. Refresh and launch again.".into());
    }
    // Preserve metadata edits made before acquiring the profile lock.
    if !install { checked_exe(dir,&fresh,exe)?; validate_launch_settings(settings(&fresh))?; }
    if runtime.id!=fresh.runtime_id { return Err("Runtime changed before launch".into()); }
    Ok(fresh)
}
fn lock_profile(dir:&Path)->Result<File,String>{
    let file=OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(dir.join("profile.lock")).map_err(|e|e.to_string())?;
    file.try_lock().map_err(|_|"Another Proton Medic instance is changing this profile")?;Ok(file)
}
fn spawn(dir:PathBuf,record:Record,r:Runtime,exe:PathBuf,install:bool)->Result<Game,String>{
    let activity=crate::coordination::activity()?;
    let mut jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;
    if jobs.contains(&record.id){return Err("This game's installer or game is already running in this app session".into());}
    let _profile_lock=lock_profile(&dir)?;
    let mut record=locked_record(&dir,&record,&r,&exe,install)?;
    if jobs.len()>=4{return Err("Close an installer/game before starting more (four active jobs maximum)".into());}
    let run=stamp()?;let log_root=dir.join("logs");directory(&log_root,true)?;let logs=log_root.join(&run);directory(&logs,true)?;
    if record.prefix_path.is_none(){for p in [dir.join("compatdata"),dir.join("compatdata/pfx")]{directory(&p,true)?;}}
    let prefix=prefix_path(&dir,&record);directory(&prefix,false)?;
    let prefix_lock=crate::coordination::prefix(&prefix)?;
    let stdout=OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(logs.join("stdout.log")).map_err(|e|e.to_string())?;
    let stderr=OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(logs.join("stderr.log")).map_err(|e|e.to_string())?;
    let previous_launch=record.last_launched_at.or_else(||history::latest_game_launch(&dir,record.run_id.as_deref()));
    record.last_launched_at=previous_launch;
    if !install {record.last_launched_at=SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d|d.as_millis() as u64);}
    record.boot_id=boot_id();record.run_id=Some(run);record.exit_code=None;record.state=if install{"installer-running"}else{"game-running"}.into();
    record.message=if install{"Complete the setup window, then select the installed game executable."}else{"Game started."}.into();
    history::begin(&logs,&record,&r,install)?;save(&dir,&record)?;
    let child=crate::runtime_session::spawn_runtime(&mut command_for_record(&r,&dir,&prefix,&exe,&logs,&record,install));
    let mut child=match child{Ok(c)=>c,Err(e)=>{record.last_launched_at=previous_launch;record.state="start-failed".into();record.message=format!("Runtime failed to start: {e}");persist_completion(&dir,&record)?;return Ok(view(&dir,record,false));}};
    let out=child.stdout.take().ok_or("Runtime stdout unavailable")?;let err=child.stderr.take().ok_or("Runtime stderr unavailable")?;
    jobs.insert(record.id.clone());let result=view(&dir,record.clone(),true);
    std::thread::spawn(move||{
        let _activity=activity;
        let _prefix_lock=prefix_lock;
        let _profile_lock_guard=_profile_lock;
        let (status,capture_ok)=crate::runtime_session::supervise_monitored(child,out,err,stdout,stderr,Some(&logs));
        record.exit_code=status.as_ref().ok().and_then(|s|s.code());
        record.state=if status.as_ref().is_ok_and(|s|s.success()){if install{"installer-exited"}else{"game-exited"}}else{"process-failed"}.into();
        record.message=if install{"Setup process ended. Select the installed game executable to continue."}else{"Game process ended."}.into();
        if let Err(e)=status{record.message=format!("Could not observe runtime exit: {e}");}
        if !capture_ok{record.message.push_str(" Runtime output capture failed; logs may be incomplete.");}
        if let Ok(mut jobs)=JOBS.lock(){if persist_completion(&dir,&record).is_ok(){jobs.remove(&record.id);}}
    });Ok(result)
}
pub fn list()->Result<Library,String>{list_in(root()?,discover_runtimes(&crate::inventory::steam()))}
fn list_in(root:PathBuf,available_runtimes:Vec<Runtime>)->Result<Library,String>{
    let mut jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;let mut games=vec![];let mut removed_games=vec![];let mut warnings=vec![];
    let mut pending=UNSAVED.lock().map_err(|_|"Completion receipt lock unavailable")?;
    pending.retain(|id,record|{let dir=root.join(id);match history::finish(&dir,record).and_then(|_|save(&dir,record)){Ok(())=>{jobs.remove(id);false},Err(e)=>{warnings.push(format!("Completion for {} could not be saved: {e}. Keep Proton Medic open; it will retry.",record.name));true}}});
    for item in fs::read_dir(&root).map_err(|e|e.to_string())?.flatten(){let id=item.file_name().to_string_lossy().into_owned();if !valid_id(&id){continue;}match load(&root,&id){Ok((dir,mut r))=>{
            if r.state.ends_with("running") && r.boot_id.is_none(){
                let migrate=(||{let _lock=lock_profile(&dir)?;let(_,mut latest)=load(&root,&id)?;
                    if latest.state.ends_with("running") && latest.boot_id.is_none(){latest.boot_id=boot_id();save(&dir,&latest)?;}Ok::<_,String>(latest)})();
                match migrate{Ok(latest)=>r=latest,Err(e)=>warnings.push(format!("Could not record recovery state for {id}: {e}"))}
            }
            let active=jobs.contains(&id) || lock_profile(&dir).is_err();
            let hidden=r.hidden;let game=view(&dir,r,active);
            if hidden{removed_games.push(game);}else{games.push(game);}
        },Err(e)=>warnings.push(format!("Could not read managed game {id}: {e}"))}}
    games.sort_by(|a,b|a.name.cmp(&b.name));removed_games.sort_by(|a,b|a.name.cmp(&b.name));Ok(Library{runtimes:available_runtimes,games,removed_games,warnings})
}
pub fn install(name:String,installer:String,runtime_id:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve running the selected Windows installer".into());}
    if name.trim().is_empty() || name.len()>120 || name.chars().any(char::is_control){return Err("Enter a game name of 1–120 characters".into());}
    let exe=windows_exe(Path::new(&installer))?;let runtime=runtime(&runtime_id)?;let root=root()?;
    let id=stamp()?;let dir=root.join(&id);directory(&dir,true)?;
    let record=Record{id,name:name.trim().into(),runtime_id,installer:exe.to_string_lossy().into(),game_exe:None,state:"created".into(),exit_code:None,message:String::new(),run_id:None,boot_id:None,prefix_path:None,hidden:false,launch_arguments:vec![],working_directory:None,default_runtime_id:None,last_launched_at:None};save(&dir,&record)?;
    spawn(dir,record,runtime,exe,true)
}
pub fn select_executable(id:String,path:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve the installed executable selection".into());}
    let root=root()?;let jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;if jobs.contains(&id){return Err("Wait for this installer/game process to finish first".into());}
    let dir=profile_dir(&root,&id)?;let _profile_lock=lock_profile(&dir)?;let(dir,mut r)=load(&root,&id)?;require_idle(&r)?;let exe=checked_exe(&dir,&r,Path::new(&path))?;
    r.game_exe=Some(exe.to_string_lossy().into());r.state="ready-to-test".into();r.message="Executable selected. You can launch the game or create a desktop shortcut.".into();save(&dir,&r)?;Ok(view(&dir,r,false))
}
pub fn launch(id:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve opening this Windows game".into());}
    launch_in(&root()?,&id)
}
fn launch_in(root:&Path,id:&str)->Result<Game,String>{
    let(dir,r)=load(root,id)?;let exe=checked_exe(&dir,&r,Path::new(r.game_exe.as_deref().ok_or("Select the installed game's executable first")?))?;let runtime=runtime(&r.runtime_id)?;spawn(dir,r,runtime,exe,false)
}
fn validate_name(name:&str)->Result<String,String>{
    if name.trim().is_empty() || name.len()>120 || name.chars().any(char::is_control){return Err("Enter a game name of 1–120 characters".into());}
    Ok(name.trim().into())
}
pub fn register(name:String,path:String,prefix:String,runtime_id:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve adding this installed game with its existing Windows prefix".into());}
    let name=validate_name(&name)?;let exe=windows_exe(Path::new(&path))?;
    let prefix=validate_prefix(Path::new(&prefix))?;let rt=runtime(&runtime_id)?;
    if rt.kind=="proton" && prefix.file_name().is_none_or(|n|n!="pfx") {
        return Err("Proton requires the existing prefix directory named pfx inside its compatibility-data directory. Use the Wine runtime for a Wine-style prefix.".into());
    }
    let root=root()?;let _lock=lock_profile(&root)?;
    for item in fs::read_dir(&root).map_err(|e|e.to_string())?.flatten(){
        let id=item.file_name().to_string_lossy().into_owned();if !valid_id(&id){continue;}
        if let Ok((dir,r))=load(&root,&id){
            if fs::canonicalize(prefix_path(&dir,&r)).ok().as_ref()==Some(&prefix) && r.game_exe.as_deref()==exe.to_str(){
                return Err(format!("This executable is already registered as {}",r.name));
            }
        }
    }
    let id=stamp()?;let dir=root.join(&id);directory(&dir,true)?;
    let record=Record{id,name,runtime_id,installer:String::new(),game_exe:Some(exe.to_string_lossy().into()),state:"ready-to-test".into(),exit_code:None,message:"Existing game added. Launch it to test.".into(),run_id:None,boot_id:None,prefix_path:Some(prefix.to_string_lossy().into()),hidden:false,launch_arguments:vec![],working_directory:None,default_runtime_id:None,last_launched_at:None};
    save(&dir,&record)?;Ok(view(&dir,record,false))
}
#[derive(Debug,Serialize)]
pub struct ExecutableSearch { pub executables:Vec<String>,pub truncated:bool }
fn search_executables(roots:Vec<PathBuf>)->ExecutableSearch{
    let mut out=ExecutableSearch{executables:vec![],truncated:false};let mut stack:Vec<_>=roots.into_iter().rev().map(|p|(p,0)).collect();
    let mut seen=HashSet::new();let mut visited=0;
    while let Some((dir,depth))=stack.pop(){
        if depth>10{out.truncated=true;continue;}
        if directory(&dir,false).is_err(){continue;}
        let Ok(entries)=fs::read_dir(&dir) else{continue};
        for entry in entries.flatten(){
            visited+=1;if visited>12000 || out.executables.len()>=100{out.truncated=true;out.executables.sort();return out;}
            let path=entry.path();let Ok(kind)=entry.file_type() else{continue};if kind.is_symlink(){continue;}
            let name=entry.file_name().to_string_lossy().to_lowercase();
            if kind.is_dir(){
                if !["windows","programdata","internet explorer","windows media player","windows nt","steam","$recycle.bin"].contains(&name.as_str()){stack.push((path,depth+1));}
            }else if kind.is_file() && name.ends_with(".exe") && !name.starts_with("unins") && !name.starts_with("setup") && !name.starts_with("vc_redist") {
                if let Ok(exe)=windows_exe(&path){if seen.insert(exe.clone()){out.executables.push(exe.to_string_lossy().into());}}
            }
        }
    }
    out.executables.sort();out
}
pub fn find_executables(id:String)->Result<ExecutableSearch,String>{
    let(dir,r)=load(&root()?,&id)?;let mut roots=vec![prefix_path(&dir,&r).join("drive_c")];
    if let Some(parent)=Path::new(&r.installer).parent().filter(|p|p.is_absolute() && p.parent().is_some()){
        if std::env::var_os("HOME").as_deref()!=Some(parent.as_os_str()){roots.push(parent.into());}
    }
    Ok(search_executables(roots))
}
#[derive(Debug,Serialize)]
#[serde(rename_all="camelCase")]
pub struct InstallationCandidate { pub path:String, pub suggested_name:String }
#[derive(Debug,Serialize)]
pub struct InstallationReview { pub candidates:Vec<InstallationCandidate>, pub truncated:bool }
fn helper_executable(path:&str)->bool {
    let name=Path::new(path).file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
    ["setup","install","unins","vc_redist","vcredist","dxsetup","dxwebsetup","dotnet","ndp","crashreport","unitycrashhandler","ue4prereq","ueprereq"].iter().any(|prefix|name.starts_with(*prefix))
}
fn suggested_name(path:&str)->String {
    let p=Path::new(path);
    for parent in p.ancestors().skip(1).take(6) {
        let Some(name)=parent.file_name().and_then(|s|s.to_str()) else{continue};
        if std::env::var_os("HOME").as_deref()==Some(parent.as_os_str()) || ["games","program files","program files (x86)","drive_c","downloads","desktop"].contains(&name.to_lowercase().as_str()){break;}
        if ["binaries","bin","win64","win32","x64","x86"].contains(&name.to_lowercase().as_str()){continue;}
        if let Ok(name)=validate_name(name){return name;}
    }
    p.file_stem().unwrap_or_default().to_string_lossy().replace('_'," ").chars().take(120).collect()
}
pub fn review_installation(id:String)->Result<InstallationReview,String>{
    let(_,r)=load(&root()?,&id)?;
    if r.state!="installer-exited" || r.game_exe.is_some(){return Err("Wait for setup to finish before reviewing its installed game".into());}
    Ok(review_candidates(find_executables(id)?))
}
fn review_candidates(result:ExecutableSearch)->InstallationReview {
    let candidates=result.executables.into_iter().filter(|path|!helper_executable(path)).map(|path|InstallationCandidate{suggested_name:suggested_name(&path),path}).collect();
    InstallationReview{candidates,truncated:result.truncated}
}
fn confirm_installation_in(root:&Path,id:&str,name:String,path:String)->Result<Game,String>{
    let name=validate_name(&name)?;let dir=profile_dir(root,id)?;let _lock=lock_profile(&dir)?;let(_,mut r)=load(root,id)?;require_idle(&r)?;
    if r.game_exe.is_some(){return Err("This game was already configured. Refresh to review its executable.".into());}
    let exe=checked_exe(&dir,&r,Path::new(&path))?;backup_profile(&dir,&r)?;
    r.name=name;r.game_exe=Some(exe.to_string_lossy().into());r.state="ready-to-test".into();r.message="Game confirmed. Launch it when you are ready to test.".into();save(&dir,&r)?;Ok(view(&dir,r,false))
}
pub fn confirm_installation(id:String,name:String,path:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve the game name and executable".into());}
    confirm_installation_in(&root()?,&id,name,path)
}
pub fn preview_executable_icon(id:String,path:String)->Result<Option<String>,String>{
    load(&root()?,&id)?;let exe=windows_exe(Path::new(&path))?;
    Ok(crate::game_icons::executable_png(&exe).map(|bytes|crate::game_icons::data_uri(&bytes,"image/png")))
}
pub fn rename(id:String,name:String,confirmed:bool)->Result<Game,String>{
    if !confirmed{return Err("Approve renaming this game".into());}
    let name=validate_name(&name)?;let root=root()?;let dir=profile_dir(&root,&id)?;let _lock=lock_profile(&dir)?;
    let(_,mut r)=load(&root,&id)?;require_idle(&r)?;backup_profile(&dir,&r)?;r.name=name;save(&dir,&r)?;Ok(view(&dir,r,false))
}
fn backup_profile(dir:&Path,record:&Record)->Result<(),String>{
    let backup=dir.join(format!("profile-backup-{}.json",stamp()?));
    let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(backup).map_err(|e|e.to_string())?;
    file.write_all(&serde_json::to_vec_pretty(record).map_err(|e|e.to_string())?).and_then(|_|file.sync_all()).map_err(|e|e.to_string())
}
fn set_hidden_in(root:&Path,id:&str,hidden:bool)->Result<(),String>{
    let dir=profile_dir(root,id)?;let _lock=lock_profile(&dir)?;let(_,mut r)=load(root,id)?;require_idle(&r)?;
    if r.hidden!=hidden{backup_profile(&dir,&r)?;r.hidden=hidden;save(&dir,&r)?;}Ok(())
}
pub fn set_hidden(id:String,hidden:bool,confirmed:bool)->Result<(),String>{
    if !confirmed{return Err("Approve changing this game's list membership".into());}
    set_hidden_in(&root()?,&id,hidden)
}
#[derive(Serialize,Deserialize,PartialEq)]
struct IconSource{path:String,size:u64,modified:u128,found:bool}
fn write_icon_cache(dir:&Path,name:&str,bytes:&[u8])->Result<(),String>{
    let target=dir.join(name);if target.exists(){regular(&target,1024*1024)?;}
    let temp=dir.join(format!(".icon-{}.tmp",stamp()?));
    let result=(||{let mut file=OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp).map_err(|e|e.to_string())?;file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;fs::rename(&temp,&target).map_err(|e|e.to_string())})();
    if result.is_err(){let _=fs::remove_file(temp);}result
}
fn icon_path_in(root:&Path,id:&str)->Option<PathBuf>{
    let(dir,r)=load(root,id).ok()?;let path=Path::new(r.game_exe.as_deref()?);let meta=fs::metadata(path).ok()?;
    let mut source=IconSource{path:path.to_str()?.into(),size:meta.len(),modified:meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos(),found:false};
    let cache=dir.join("game-icon-source.json");let icon=dir.join("game-icon.png");
    if regular(&cache,65536).is_ok(){if let Ok(previous)=serde_json::from_slice::<IconSource>(&fs::read(&cache).ok()?){
        source.found=previous.found;
        if source==previous{if !previous.found{return None;}if regular(&icon,1024*1024).is_ok(){return Some(icon);}}
    }}
    source.found=false;
    if let Some(png)=crate::game_icons::executable_png(path){
        write_icon_cache(&dir,"game-icon.png",&png).ok()?;source.found=true;
    }
    write_icon_cache(&dir,"game-icon-source.json",&serde_json::to_vec(&source).ok()?).ok()?;
    source.found.then_some(icon)
}
pub fn icon_path(id:&str)->Option<PathBuf>{icon_path_in(&root().ok()?,id)}
pub fn icon(id:String)->Result<Option<String>,String>{
    let root=root()?;load(&root,&id)?;
    let Some(path)=icon_path_in(&root,&id) else{return Ok(None)};
    regular(&path,1024*1024)?;
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    Ok(Some(crate::game_icons::data_uri(&bytes,"image/png")))
}
pub fn shortcut_profile(id:&str)->Result<(String,PathBuf),String>{
    let root=root()?;let(dir,r)=load(&root,id)?;
    checked_exe(&dir,&r,Path::new(r.game_exe.as_deref().ok_or("Select the game executable before creating a shortcut")?))?;
    runtime(&r.runtime_id)?;Ok((r.name,root))
}
pub fn launch_shortcut(root:&Path,id:&str)->Result<(),String>{
    directory(root,false)?;
    let started=launch_in(root,id)?;
    if started.state=="start-failed"{return Err(started.message);}
    loop{
        {
            let jobs=JOBS.lock().map_err(|_|"Process registry unavailable")?;
            if !jobs.contains(id){break;}
            if UNSAVED.lock().map_err(|_|"Completion receipt unavailable")?.contains_key(id){return Err("Game ended but its completion receipt could not be saved. Check profile permissions.".into());}
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    let(_,done)=load(root,id)?;
    if done.state=="process-failed"{return Err(format!("{} exited with status {:?}. Open Proton Medic to read the runtime logs.",done.name,done.exit_code));}
    Ok(())
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
    fn record(id:&str)->Record{Record{id:id.into(),name:"Fixture".into(),runtime_id:"fixture".into(),installer:String::new(),game_exe:None,state:"created".into(),exit_code:None,message:String::new(),run_id:None,boot_id:None,prefix_path:None,hidden:false,launch_arguments:vec![],working_directory:None,default_runtime_id:None,last_launched_at:None}}
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
    #[test] fn game_selection_preserves_prefix_for_external_game(){
        let f=Fixture::new();fs::create_dir_all(f.0.join("compatdata/pfx/drive_c/Game")).unwrap();
        let outside=f.0.join("outside.exe");pe(&outside);
        assert_eq!(checked_exe(&f.0,&record("123"),&outside).unwrap(),outside);
        assert_eq!(prefix_path(&f.0,&record("123")),f.0.join("compatdata/pfx"));
        assert!(checked_exe(&f.0,&record("123"),Path::new("/missing.exe")).is_err());
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
        let r=Runtime{proton:None,id:"fixture".into(),name:"Fixture".into(),kind:"proton".into(),binary:PathBuf::from("/fixture/proton"),steam:Some(PathBuf::from("/fixture/Steam"))};
        let cmd=command(&r,Path::new("/managed/123"),Path::new("/Downloads/Game Setup.exe"),Path::new("/managed/123/logs/1"));
        assert_eq!(cmd.get_args().collect::<Vec<_>>(),vec![std::ffi::OsStr::new("run"),std::ffi::OsStr::new("/Downloads/Game Setup.exe")]);
        let vars:HashMap<_,_>=cmd.get_envs().collect();assert_eq!(vars[std::ffi::OsStr::new("WINEPREFIX")],Some(std::ffi::OsStr::new("/managed/123/compatdata/pfx")));
        assert_eq!(vars[std::ffi::OsStr::new("PROTON_LOG")],Some(std::ffi::OsStr::new("0")));
    }
    #[test] fn output_capture_is_bounded(){let f=Fixture::new();let file=f.0.join("log");crate::runtime_session::capture_fixture(std::io::Cursor::new(vec![b'x';3*1024*1024]),OpenOptions::new().read(true).write(true).create_new(true).open(&file).unwrap()).unwrap();assert_eq!(fs::metadata(file).unwrap().len(),2*1024*1024);}
    #[test] fn fake_runtime_executes_installer_protocol_and_persists_exit(){
        let f=Fixture::new();let id=stamp().unwrap();let dir=f.0.join(&id);directory(&dir,true).unwrap();let exe=f.0.join("Setup With Spaces.exe");pe(&exe);
        let fake=f.0.join("fake-wine");fs::write(&fake,"#!/bin/sh\nmkdir -p \"$WINEPREFIX/drive_c/Game\"\ncp \"$1\" \"$WINEPREFIX/drive_c/Game/game.exe\"\nprintf 'unhandled exception fixture\\n' >&2\nexit 7\n").unwrap();fs::set_permissions(&fake,fs::Permissions::from_mode(0o700)).unwrap();
        let r=Runtime{proton:None,id:"fixture".into(),name:"Fake Wine".into(),kind:"wine".into(),binary:fake,steam:None};let rec=record(&id);save(&dir,&rec).unwrap();
        let started=spawn(dir.clone(),rec,r,exe,true).unwrap();assert_eq!(started.state,"installer-running");
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
        while JOBS.lock().unwrap().contains(&id){assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(10));}
        let(_,done)=load(&f.0,&id).unwrap();assert_eq!(done.exit_code,Some(7));assert_eq!(done.state,"process-failed");
        assert!(checked_exe(&dir,&done,&dir.join("compatdata/pfx/drive_c/Game/game.exe")).is_ok());
        let log=dir.join("logs").join(done.run_id.as_ref().unwrap()).join("stderr.log");assert!(fs::read_to_string(log).unwrap().contains("unhandled exception"));
        let history=history::list_in(&dir,&done,false).unwrap();assert_eq!(history.entries[0].kind,"installer");assert_eq!(history.entries[0].exit_code,Some(7));assert!(!history.entries[0].summaries.is_empty());
    }
    #[test] fn missing_runtime_is_recorded_as_failure_without_claiming_installation(){
        let f=Fixture::new();let id=stamp().unwrap();let dir=f.0.join(&id);directory(&dir,true).unwrap();let exe=f.0.join("setup.exe");pe(&exe);
        let r=Runtime{proton:None,id:"none".into(),name:"Missing".into(),kind:"wine".into(),binary:f.0.join("absent"),steam:None};
        let mut rec=record(&id);rec.runtime_id="none".into();save(&dir,&rec).unwrap();let result=spawn(dir.clone(),rec,r,exe,true).unwrap();assert_eq!(result.state,"start-failed");assert!(result.game_exe.is_none());assert!(load(&f.0,&id).is_ok());
        let (_,done)=load(&f.0,&id).unwrap();let history=history::list_in(&dir,&done,false).unwrap();assert_eq!(history.entries[0].state,"start-failed");assert_eq!(history.entries[0].exit_code,None);
    }
    #[test] fn unknown_persisted_runs_are_blocked_until_a_proven_reboot(){
        let mut r=record("123");r.state="game-running".into();assert!(require_idle(&r).is_err());
        r.boot_id=boot_id();assert!(require_idle(&r).is_err());
        if boot_id().is_some(){r.boot_id=Some("previous-boot".into());assert!(require_idle(&r).is_ok());}
    }
    #[test] fn tail_capture_preserves_late_failure(){
        let f=Fixture::new();let p=f.0.join("tail");let mut data=vec![b'x';3*1024*1024];data.extend_from_slice(b"late crash marker");
        crate::runtime_session::capture_fixture(std::io::Cursor::new(data),OpenOptions::new().read(true).write(true).create_new(true).open(&p).unwrap()).unwrap();
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

    #[test] fn legacy_profiles_load_without_an_external_prefix_field(){
        let mut json=serde_json::to_value(record("123")).unwrap();json.as_object_mut().unwrap().remove("prefixPath");
        let r:Record=serde_json::from_value(json).unwrap();assert!(r.prefix_path.is_none());
        assert_eq!(prefix_path(Path::new("/managed/123"),&r),Path::new("/managed/123/compatdata/pfx"));
    }
    #[test] fn imported_game_uses_existing_prefix_in_runtime_environment(){
        let mut r=record("123");r.prefix_path=Some("/existing/My Game/pfx".into());
        let rt=Runtime{proton:None,id:"fixture".into(),name:"Fixture".into(),kind:"proton".into(),binary:PathBuf::from("/fixture/proton"),steam:None};
        let prefix=prefix_path(Path::new("/managed/123"),&r);
        let cmd=command_with_prefix(&rt,Path::new("/managed/123"),&prefix,Path::new("/games/Wukong/b1.exe"),Path::new("/logs"));
        let vars:HashMap<_,_>=cmd.get_envs().collect();
        assert_eq!(vars[std::ffi::OsStr::new("WINEPREFIX")],Some(std::ffi::OsStr::new("/existing/My Game/pfx")));
        assert_eq!(vars[std::ffi::OsStr::new("STEAM_COMPAT_DATA_PATH")],Some(std::ffi::OsStr::new("/existing/My Game")));
    }
    #[test] fn existing_prefix_requires_registry_and_c_drive(){
        let f=Fixture::new();assert!(validate_prefix(&f.0).is_err());
        fs::create_dir(f.0.join("drive_c")).unwrap();assert!(validate_prefix(&f.0).is_err());
        fs::write(f.0.join("system.reg"),b"WINE REGISTRY Version 2").unwrap();assert!(validate_prefix(&f.0).is_ok());
        assert!(validate_prefix(Path::new("relative")).is_err());
    }
    #[test] fn search_finds_external_game_and_skips_symlinks_and_system_files(){
        let f=Fixture::new();fs::create_dir(f.0.join("Windows")).unwrap();pe(&f.0.join("Windows/system.exe"));
        pe(&f.0.join("b1.exe"));pe(&f.0.join("setup.exe"));
        std::os::unix::fs::symlink(f.0.join("b1.exe"),f.0.join("linked.exe")).unwrap();
        let result=search_executables(vec![f.0.clone(),f.0.clone()]);
        assert_eq!(result.executables,vec![f.0.join("b1.exe").to_string_lossy().into_owned()]);assert!(!result.truncated);
    }
    #[test] fn registration_and_renaming_require_consent_before_io(){
        assert!(register("Game".into(),"/none.exe".into(),"/none".into(),"none".into(),false).unwrap_err().contains("Approve"));
        assert!(rename("123".into(),"Game".into(),false).unwrap_err().contains("Approve"));
    }

    #[test] fn removal_and_restore_preserve_game_files_prefixes_and_backups(){
        let f=Fixture::new();let id="123-456";let dir=f.0.join(id);directory(&dir,true).unwrap();
        let data=dir.join("compatdata/pfx/drive_c/Game/save.dat");fs::create_dir_all(data.parent().unwrap()).unwrap();fs::write(&data,b"saved progress").unwrap();save(&dir,&record(id)).unwrap();
        set_hidden_in(&f.0,id,true).unwrap();assert!(load(&f.0,id).unwrap().1.hidden);
        let hidden=list_in(f.0.clone(),vec![]).unwrap();assert!(hidden.games.is_empty());assert_eq!(hidden.removed_games.len(),1);
        assert_eq!(fs::read(&data).unwrap(),b"saved progress");
        set_hidden_in(&f.0,id,false).unwrap();let restored=list_in(f.0.clone(),vec![]).unwrap();assert_eq!(restored.games.len(),1);assert!(restored.removed_games.is_empty());
        assert_eq!(fs::read(&data).unwrap(),b"saved progress");
        assert_eq!(fs::read_dir(&dir).unwrap().flatten().filter(|e|e.file_name().to_string_lossy().starts_with("profile-backup-")).count(),2);
    }
    #[test] fn cannot_remove_a_running_or_locked_profile(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let mut r=record(id);r.state="game-running".into();r.boot_id=boot_id();save(&dir,&r).unwrap();
        assert!(set_hidden_in(&f.0,id,true).is_err());assert!(!load(&f.0,id).unwrap().1.hidden);
        r.state="ready-to-test".into();save(&dir,&r).unwrap();let _guard=lock_profile(&dir).unwrap();assert!(set_hidden_in(&f.0,id,true).is_err());
    }
    #[test] fn old_profiles_are_visible_without_the_hidden_field(){
        let mut json=serde_json::to_value(record("123")).unwrap();json.as_object_mut().unwrap().remove("hidden");
        let r:Record=serde_json::from_value(json).unwrap();assert!(!r.hidden);
    }
    #[test] fn changing_list_membership_requires_explicit_consent(){
        assert!(set_hidden("123".into(),true,false).unwrap_err().contains("Approve"));
        assert!(set_hidden("123".into(),false,false).unwrap_err().contains("Approve"));
    }
    #[test] fn missing_or_changed_icon_source_does_not_reuse_a_stale_icon(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let mut r=record(id);let exe=f.0.join("empty.exe");pe(&exe);r.game_exe=Some(exe.to_string_lossy().into());save(&dir,&r).unwrap();
        fs::write(dir.join("game-icon.png"),b"stale image").unwrap();assert!(icon_path_in(&f.0,id).is_none());
        let source:IconSource=serde_json::from_slice(&fs::read(dir.join("game-icon-source.json")).unwrap()).unwrap();assert!(!source.found);
        fs::remove_file(exe).unwrap();assert!(icon_path_in(&f.0,id).is_none());
    }

    #[test] fn launch_uses_latest_locked_name_and_hidden_state() {
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();
        let previous=record(id);let mut latest=previous.clone();latest.name="Renamed".into();latest.hidden=true;save(&dir,&latest).unwrap();
        let rt=Runtime{proton:None,id:"fixture".into(),name:"Fixture".into(),kind:"wine".into(),binary:PathBuf::from("/fixture"),steam:None};
        let _lock=lock_profile(&dir).unwrap();let fresh=locked_record(&dir,&previous,&rt,Path::new("/setup.exe"),true).unwrap();
        assert_eq!(fresh.name,"Renamed");assert!(fresh.hidden);
    }
    #[test] fn launch_stops_if_executable_runtime_or_prefix_changed_before_lock() {
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let previous=record(id);
        let rt=Runtime{proton:None,id:"fixture".into(),name:"Fixture".into(),kind:"wine".into(),binary:PathBuf::from("/fixture"),steam:None};
        for mut latest in [previous.clone(),previous.clone(),previous.clone()] .into_iter().enumerate() {
            match latest.0 {0=>latest.1.game_exe=Some("/different.exe".into()),1=>latest.1.runtime_id="new".into(),_=>latest.1.prefix_path=Some("/new/pfx".into())}
            save(&dir,&latest.1).unwrap();assert!(locked_record(&dir,&previous,&rt,Path::new("/setup.exe"),true).unwrap_err().contains("settings changed"));
        }
    }

    #[test] fn umu_launch_preserves_selected_proton_prefix_and_literal_arguments(){
        let rt=Runtime{proton:Some(PathBuf::from("/fixture/GE-Proton")),id:"fixture".into(),name:"GE".into(),kind:"proton".into(),binary:PathBuf::from("/fixture/umu-run"),steam:None};
        let f=Fixture::new();let mut r=record("123");r.launch_arguments=vec!["two words".into(),"$HOME".into()];
        let cmd=command_for_record(&rt,&f.0,Path::new("/prefix/pfx"),Path::new("/games/game.exe"),&f.0,&r,false);
        assert_eq!(cmd.get_program(),"/fixture/umu-run");assert_eq!(cmd.get_args().collect::<Vec<_>>(),vec!["/games/game.exe","two words","$HOME"]);
        let env=cmd.get_envs().collect::<std::collections::HashMap<_,_>>();assert_eq!(env.get(std::ffi::OsStr::new("PROTONPATH")),Some(&Some(std::ffi::OsStr::new("/fixture/GE-Proton"))));assert_eq!(env.get(std::ffi::OsStr::new("WINEPREFIX")),Some(&Some(std::ffi::OsStr::new("/prefix/pfx"))));
    }
    fn fixture_runtime(id:&str)->Result<Runtime,String>{Ok(Runtime{proton:None,id:id.into(),name:id.into(),kind:"wine".into(),binary:PathBuf::from("/fixture/wine"),steam:None})}
    #[test] fn launch_settings_backup_reset_and_stale_edit_preserve_profile_and_prefix(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let mut r=record(id);r.game_exe=Some("/game.exe".into());save(&dir,&r).unwrap();
        let original=settings(&r);let changed=LaunchSettings{runtime_id:"other".into(),arguments:vec!["--name=two words".into(),"$(touch /tmp/never)".into()],working_directory:Some(f.0.to_string_lossy().into())};
        save_settings_in(&f.0,id,Some(changed.clone()),original.clone(),fixture_runtime).unwrap();
        let(_,saved)=load(&f.0,id).unwrap();assert_eq!(settings(&saved),changed);assert_eq!(saved.default_runtime_id.as_deref(),Some("fixture"));assert_eq!(saved.game_exe,r.game_exe);assert_eq!(saved.prefix_path,r.prefix_path);
        assert!(save_settings_in(&f.0,id,Some(original.clone()),original.clone(),fixture_runtime).unwrap_err().contains("another window"));
        save_settings_in(&f.0,id,None,changed,fixture_runtime).unwrap();assert_eq!(settings(&load(&f.0,id).unwrap().1),original);
        assert_eq!(fs::read_dir(&dir).unwrap().flatten().filter(|e|e.file_name().to_string_lossy().starts_with("profile-backup-")).count(),2);
    }
    #[test] fn launch_arguments_are_literal_and_installer_does_not_inherit_game_options(){
        let rt=fixture_runtime("fixture").unwrap();let mut r=record("123");r.launch_arguments=vec!["two words".into(),"; touch forbidden".into(),"$HOME".into()];r.working_directory=Some("/chosen/work".into());
        let cmd=command_for_record(&rt,Path::new("/managed/123"),Path::new("/prefix"),Path::new("/game.exe"),Path::new("/logs"),&r,false);
        assert_eq!(cmd.get_args().map(|s|s.to_str().unwrap()).collect::<Vec<_>>(),vec!["/game.exe","two words","; touch forbidden","$HOME"]);assert_eq!(cmd.get_current_dir(),Some(Path::new("/chosen/work")));
        let installer=command_for_record(&rt,Path::new("/managed/123"),Path::new("/prefix"),Path::new("/setup.exe"),Path::new("/logs"),&r,true);
        assert_eq!(installer.get_args().count(),1);assert_eq!(installer.get_current_dir(),Some(Path::new("/")));
    }
    #[test] fn rejects_unbounded_arguments_control_characters_and_invalid_working_directories(){
        let base=LaunchSettings{runtime_id:"fixture".into(),arguments:vec![],working_directory:None};
        for args in [vec!["x".into();65],vec!["x".repeat(4097)],vec!["x".repeat(4096);5],vec!["x\0y".into()],vec!["x\ny".into()]] {let mut value=base.clone();value.arguments=args;assert!(validate_launch_settings(value).is_err());}
        for path in ["relative/path","/missing/proton-medic-work-directory"] {let mut value=base.clone();value.working_directory=Some(path.into());assert!(validate_launch_settings(value).is_err());}
        let f=Fixture::new();let link=f.0.join("link");std::os::unix::fs::symlink(&f.0,&link).unwrap();let mut value=base;value.working_directory=Some(link.to_string_lossy().into());assert!(validate_launch_settings(value).is_err());
    }
    #[test] fn settings_edits_cannot_reuse_running_or_locked_profiles(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let mut r=record(id);r.state="game-running".into();r.boot_id=boot_id();save(&dir,&r).unwrap();let before=settings(&r);
        assert!(save_settings_in(&f.0,id,None,before.clone(),fixture_runtime).is_err());r.state="game-exited".into();save(&dir,&r).unwrap();let _lock=lock_profile(&dir).unwrap();assert!(save_settings_in(&f.0,id,None,before,fixture_runtime).is_err());
    }
    #[test] fn review_filters_prerequisites_and_suggests_game_folder_names(){
        let review=review_candidates(ExecutableSearch{executables:vec!["/Games/Black Myth Wukong/Binaries/Win64/b1.exe".into(),"/Games/Black Myth Wukong/DXSetup.exe".into(),"/Games/Black Myth Wukong/CrashReportClient.exe".into(),"/Games/Black Myth Wukong/installer.exe".into()],truncated:true});
        assert_eq!(review.candidates.len(),1);assert_eq!(review.candidates[0].suggested_name,"Black Myth Wukong");assert!(review.truncated);assert_eq!(suggested_name("/home/player/Downloads/game.exe"),"game");
    }
    #[test] fn installation_confirmation_is_atomic_backed_up_and_never_launches_a_game(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();fs::create_dir_all(dir.join("compatdata/pfx/drive_c")).unwrap();let exe=f.0.join("b1.exe");pe(&exe);let mut r=record(id);r.state="installer-exited".into();save(&dir,&r).unwrap();
        assert!(confirm_installation_in(&f.0,id,"New game".into(),"/missing.exe".into()).is_err());assert_eq!(load(&f.0,id).unwrap().1.name,"Fixture");
        let game=confirm_installation_in(&f.0,id,"Wukong".into(),exe.to_string_lossy().into()).unwrap();assert_eq!(game.state,"ready-to-test");assert_eq!(game.name,"Wukong");assert!(!JOBS.lock().unwrap().contains(id));
        assert!(confirm_installation_in(&f.0,id,"Changed".into(),exe.to_string_lossy().into()).is_err());assert_eq!(load(&f.0,id).unwrap().1.name,"Wukong");
        assert!(fs::read_dir(&dir).unwrap().flatten().any(|e|e.file_name().to_string_lossy().starts_with("profile-backup-")));
    }
    #[test] fn new_game_configuration_commands_require_approval_before_io(){
        let value=LaunchSettings{runtime_id:"none".into(),arguments:vec![],working_directory:None};
        assert!(save_launch_settings("none".into(),value.clone(),value.clone(),false).unwrap_err().contains("Approve"));
        assert!(reset_launch_settings("none".into(),value,false).unwrap_err().contains("Approve"));assert!(confirm_installation("none".into(),"Game".into(),"/none.exe".into(),false).unwrap_err().contains("Approve"));
    }
    #[test] fn legacy_profiles_default_to_original_launch_behavior(){
        let mut json=serde_json::to_value(record("123")).unwrap();for key in ["launchArguments","workingDirectory","defaultRuntimeId"] {json.as_object_mut().unwrap().remove(key);}
        let r:Record=serde_json::from_value(json).unwrap();assert!(r.launch_arguments.is_empty());assert!(r.working_directory.is_none());assert!(r.default_runtime_id.is_none());
    }
    #[test] fn switching_to_proton_cannot_move_or_reuse_a_wine_style_prefix(){
        let f=Fixture::new();let id="123";let dir=f.0.join(id);directory(&dir,true).unwrap();let prefix=f.0.join("wine-prefix");fs::create_dir(&prefix).unwrap();fs::write(prefix.join("save.dat"),b"progress").unwrap();let mut r=record(id);r.prefix_path=Some(prefix.to_string_lossy().into());save(&dir,&r).unwrap();
        let before=settings(&r);let value=LaunchSettings{runtime_id:"proton".into(),arguments:vec![],working_directory:None};
        assert!(save_settings_in(&f.0,id,Some(value),before.clone(),|id|{let mut rt=fixture_runtime(id)?;rt.kind="proton".into();Ok(rt)}).is_err());
        assert_eq!(settings(&load(&f.0,id).unwrap().1),before);assert_eq!(fs::read(prefix.join("save.dat")).unwrap(),b"progress");
    }

}
