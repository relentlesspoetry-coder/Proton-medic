//! Desktop launchers reuse the saved runtime and prefix through the native CLI.
use std::{fs::{self,OpenOptions},io::Write,os::unix::fs::OpenOptionsExt,path::{Path,PathBuf}};

fn value(s:&str)->Result<String,String>{
    if s.chars().any(char::is_control){return Err("Shortcut fields cannot contain control characters".into());}
    Ok(s.replace('\\',"\\\\"))
}
fn argument(s:&str)->Result<String,String>{
    if s.chars().any(char::is_control){return Err("Shortcut paths cannot contain control characters".into());}
    // Escape Exec quoting first, then the desktop-entry string layer.
    let mut quoted=String::new();
    for c in s.chars(){match c {'\\'|'"'|'`'|'$'=>{quoted.push('\\');quoted.push(c);},'%'=>quoted.push_str("%%"),_=>quoted.push(c)}}
    Ok(format!("\"{}\"",quoted.replace('\\',"\\\\")))
}
fn entry(name:&str,binary:&Path,root:&Path,id:&str,icon:&Path)->Result<String,String>{
    let binary=binary.to_str().ok_or("Application path is not UTF-8")?;
    if binary.contains('='){return Err("The application path cannot contain = in a desktop launcher".into());}
    Ok(format!("[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nComment=Launch with the saved Proton Medic runtime and prefix\nExec={} --launch-windows-game {} --profile-root {}\nIcon={}\nTerminal=false\nCategories=Game;\nStartupNotify=false\n",value(name)?,argument(binary)?,argument(id)?,argument(root.to_str().ok_or("Profile path is not UTF-8")?)?,value(icon.to_str().ok_or("Icon path is not UTF-8")?)?))
}
fn create_file(path:&Path,bytes:&[u8],mode:u32)->Result<(),String>{
    match OpenOptions::new().write(true).create_new(true).mode(mode).open(path){
        Ok(mut file)=>{
            if let Err(e)=file.write_all(bytes).and_then(|_|file.sync_all()){let _=fs::remove_file(path);return Err(e.to_string());}Ok(())
        },
        Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{
            let m=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
            if m.is_file() && !m.file_type().is_symlink() && m.len()==bytes.len() as u64 && fs::read(path).map_err(|e|e.to_string())?==bytes{Ok(())}
            else{Err(format!("{} already exists with different contents; it was left unchanged",path.display()))}
        },Err(e)=>Err(e.to_string())
    }
}
fn write_launcher(path:&Path,content:&[u8],backups:&Path)->Result<(),String>{
    if !path.exists(){return create_file(path,content,0o700);}
    let meta=fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len()>65536{return Err("Existing shortcut is not a regular launcher; it was left unchanged".into());}
    let mut previous=vec![];
    use std::io::Read;
    OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW|libc::O_NONBLOCK).open(path).map_err(|e|e.to_string())?.take(65537).read_to_end(&mut previous).map_err(|e|e.to_string())?;
    if previous==content{return Ok(());}
    let before=std::str::from_utf8(&previous).map_err(|_|"Existing shortcut is not UTF-8")?;
    let after=std::str::from_utf8(content).map_err(|_|"Invalid launcher content")?;
    let exec=|text:&str|text.lines().find(|line|line.starts_with("Exec=")).map(str::to_owned);
    if exec(before).is_none() || exec(before)!=exec(after){return Err("Existing shortcut launches a different command; it was left unchanged".into());}
    if !backups.exists(){use std::os::unix::fs::DirBuilderExt;fs::DirBuilder::new().mode(0o700).create(backups).map_err(|e|e.to_string())?;}
    let meta=fs::symlink_metadata(backups).map_err(|e|e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink(){return Err("Shortcut backup directory must not be a symbolic link".into());}
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let backup=backups.join(format!("launcher-{stamp}.desktop"));create_file(&backup,&previous,0o600)?;
    let temporary=path.with_extension(format!("desktop-{stamp}.tmp"));create_file(&temporary,content,0o700)?;
    if let Err(error)=fs::rename(&temporary,path){let _=fs::remove_file(temporary);return Err(error.to_string());}Ok(())
}
fn desktop_dir()->Result<PathBuf,String>{
    let home=std::env::var_os("HOME").map(PathBuf::from).ok_or("Home directory unavailable")?;
    let configured=crate::font_runtime::child_command("xdg-user-dir").arg("DESKTOP").output().ok().filter(|o|o.status.success()).and_then(|o|String::from_utf8(o.stdout).ok()).map(|s|PathBuf::from(s.trim_end_matches(['\r','\n'])));
    let desktop=configured.unwrap_or_else(||home.join("Desktop"));
    if !desktop.is_absolute() || desktop==home || !desktop.is_dir(){return Err("No desktop folder is configured. Enable a desktop folder before creating a desktop shortcut.".into());}
    Ok(desktop)
}
pub fn create(id:String,confirmed:bool)->Result<Vec<String>,String>{
    if !confirmed{return Err("Approve creating this game's desktop and applications-menu shortcuts".into());}
    let(name,root)=crate::windows_games::shortcut_profile(&id)?;
    let desktop=desktop_dir()?;
    let app_data=root.parent().ok_or("Application data unavailable")?;
    let applications=app_data.parent().ok_or("Data directory unavailable")?.join("applications");
    fs::create_dir_all(&applications).map_err(|e|e.to_string())?;
    let icon=if let Some(path)=crate::windows_games::icon_path(&id){path}else{
        let path=app_data.join("game-shortcut-icon.png");
        create_file(&path,include_bytes!("../icons/128x128.png"),0o600)?;path
    };
    let content=entry(&name,&crate::portable::executable()?,&root,&id,&icon)?;
    let filename=format!("proton-medic-game-{id}.desktop");let mut paths=vec![];
    for dir in [applications,desktop]{
        let path=dir.join(&filename);write_launcher(&path,content.as_bytes(),&root.join(&id).join("shortcut-backups"))?;
        paths.push(path.to_string_lossy().into());
        // GNOME may still ask the player to Allow Launching on first use.
        let _=crate::font_runtime::child_command("gio").args(["set"]).arg(&path).args(["metadata::trusted","true"]).output();
    }
    Ok(paths)
}
/// Returns None for a normal GUI start, or an exit status for shortcut mode.
pub fn handle_cli(args:Vec<String>)->Option<i32>{
    if !args.iter().any(|a|a=="--launch-windows-game"){return None;}
    let result=if args.len()==5 && args[1]=="--launch-windows-game" && args[3]=="--profile-root" && Path::new(&args[4]).is_absolute(){
        crate::windows_games::launch_shortcut(Path::new(&args[4]),&args[2])
    }else{Err("Invalid game shortcut arguments. Recreate the shortcut in Proton Medic.".into())};
    match result{Ok(())=>Some(0),Err(error)=>{
        eprintln!("Proton Medic: {error}");
        let _=crate::font_runtime::child_command("notify-send").args(["--app-name=Proton Medic","--","Game could not be launched",&error]).output();
        Some(1)
    }}
}
#[cfg(test)]
mod tests{
    use super::*;
    use std::process::Command;
    #[test] fn entry_uses_profile_id_and_quotes_paths_without_a_shell(){
        let text=entry("Game & Friends",Path::new("/opt/Proton Medic/proton-medic"),Path::new("/data/My Games"),"123-456",Path::new("/icon.png")).unwrap();
        assert!(text.contains("Exec=\"/opt/Proton Medic/proton-medic\" --launch-windows-game \"123-456\" --profile-root \"/data/My Games\""));
        assert!(text.contains("Terminal=false"));assert!(!text.contains("sh -c"));
    }
    #[test] fn escapes_reserved_characters_and_rejects_newline_injection(){
        assert_eq!(argument("$HOME%f`x`\\\"" ).unwrap(),"\"\\\\$HOME%%f\\\\`x\\\\`\\\\\\\\\\\\\"\"");
        assert!(entry("Game\nExec=evil",Path::new("/bin/app"),Path::new("/data"),"123",Path::new("/icon")).is_err());
        assert!(argument("path\nline").is_err());
    }
    #[test] fn shortcut_files_never_replace_existing_files_or_symlinks(){
        let root=std::env::temp_dir().join(format!("pm-shortcut-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).unwrap();
        let path=root.join("game.desktop");create_file(&path,b"original",0o700).unwrap();assert!(create_file(&path,b"changed",0o700).is_err());assert_eq!(fs::read(&path).unwrap(),b"original");
        let link=root.join("link.desktop");std::os::unix::fs::symlink(&path,&link).unwrap();assert!(create_file(&link,b"original",0o700).is_err());fs::remove_dir_all(root).unwrap();
    }
    #[test] fn consent_is_required_before_shortcut_io(){assert!(create("invalid".into(),false).unwrap_err().contains("Approve"));}
    #[test] fn malformed_shortcut_cannot_start_a_game(){assert_eq!(handle_cli(vec!["app".into()]),None);}
    #[test] fn desktop_entry_round_trips_arguments_through_gio(){
        let root=std::env::temp_dir().join(format!("pm-gio-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).unwrap();
        let binary=root.join("Fake Launcher");let receipt=root.join("arguments.json");
        fs::write(&binary,format!("#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\nPath({:?}).write_text(json.dumps(sys.argv[1:]))\n",receipt.to_str().unwrap())).unwrap();
        use std::os::unix::fs::PermissionsExt;fs::set_permissions(&binary,fs::Permissions::from_mode(0o700)).unwrap();
        let profile=root.join("My Games %f $HOME `literal` \\");
        let desktop=root.join("fixture.desktop");fs::write(&desktop,entry("Fixture",&binary,&profile,"123-456",Path::new("/icon.png")).unwrap()).unwrap();
        let output=Command::new("gio").arg("launch").arg(&desktop).output().expect("gio is required for desktop entry integration tests");
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
        while !receipt.exists(){assert!(std::time::Instant::now()<deadline);std::thread::sleep(std::time::Duration::from_millis(10));}
        let args:Vec<String>=serde_json::from_str(&fs::read_to_string(receipt).unwrap()).unwrap();
        assert_eq!(args,vec!["--launch-windows-game".to_owned(),"123-456".into(),"--profile-root".into(),profile.to_string_lossy().into_owned()]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test] fn owned_shortcut_refresh_backs_up_its_previous_name_and_icon(){
        let root=std::env::temp_dir().join(format!("pm-refresh-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&root).unwrap();
        let path=root.join("game.desktop");let backup=root.join("backups");
        let old=entry("Old name",Path::new("/bin/app"),Path::new("/data"),"123",Path::new("/old.png")).unwrap();
        let new=entry("New name",Path::new("/bin/app"),Path::new("/data"),"123",Path::new("/game.png")).unwrap();
        create_file(&path,old.as_bytes(),0o700).unwrap();write_launcher(&path,new.as_bytes(),&backup).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(),new);let saved=fs::read_dir(&backup).unwrap().next().unwrap().unwrap().path();assert_eq!(fs::read_to_string(saved).unwrap(),old);
        assert!(write_launcher(&path,b"[Desktop Entry]\nExec=/different-app\n",&backup).is_err());assert_eq!(fs::read_to_string(&path).unwrap(),new);fs::remove_dir_all(root).unwrap();
    }

}
