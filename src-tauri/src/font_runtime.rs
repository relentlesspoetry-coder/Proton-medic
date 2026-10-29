//! App-local font configuration, prepared before GTK/WebKit starts any threads.
use std::{ffi::OsString,fs,os::unix::fs::DirBuilderExt,path::{Path,PathBuf},process::Command,sync::OnceLock,time::{SystemTime,UNIX_EPOCH}};
static ORIGINAL:OnceLock<Option<OsString>>=OnceLock::new();
pub struct FontRuntime(PathBuf);
impl Drop for FontRuntime {fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
fn xml(s:&str)->String{s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&apos;")}
fn prepare(base:&Path)->Result<FontRuntime,String>{
    let stamp=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos();
    let dir=base.join(format!("proton-medic-fonts-{}-{stamp}",std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&dir).map_err(|e|format!("Cannot create private font directory: {e}"))?;
    let guard=FontRuntime(dir);
    fs::create_dir(guard.0.join("fonts")).map_err(|e|e.to_string())?;
    fs::create_dir(guard.0.join("cache")).map_err(|e|e.to_string())?;
    fs::write(guard.0.join("fonts/DejaVuSans.ttf"),include_bytes!("../fonts/DejaVuSans.ttf")).map_err(|e|e.to_string())?;
    fs::write(guard.0.join("fonts/DejaVuSans-Bold.ttf"),include_bytes!("../fonts/DejaVuSans-Bold.ttf")).map_err(|e|e.to_string())?;
    let fonts=xml(&guard.0.join("fonts").to_string_lossy());let cache=xml(&guard.0.join("cache").to_string_lossy());
    fs::write(guard.0.join("fonts.conf"),format!("<?xml version=\"1.0\"?><!DOCTYPE fontconfig SYSTEM \"urn:fontconfig:fonts.dtd\"><fontconfig><dir>{fonts}</dir><cachedir>{cache}</cachedir></fontconfig>")).map_err(|e|e.to_string())?;
    Ok(guard)
}
/// Must only run once, before the Tauri runtime creates threads.
pub fn initialize()->Result<Option<FontRuntime>,String>{
    if std::env::var_os("PROTON_MEDIC_SYSTEM_FONTS").as_deref()==Some(std::ffi::OsStr::new("1")){return Ok(None);}
    let guard=prepare(&std::env::temp_dir())?;
    ORIGINAL.set(std::env::var_os("FONTCONFIG_FILE")).map_err(|_|"Font runtime already initialized")?;
    std::env::set_var("FONTCONFIG_FILE",guard.0.join("fonts.conf"));
    // Fontconfig populates this new private cache on first use; no shared cache is imported.
    eprintln!("[Proton Medic startup] Isolated bundled fonts enabled");
    Ok(Some(guard))
}
fn restore(command:&mut Command,original:Option<&OsString>){match original{Some(value)=>{command.env("FONTCONFIG_FILE",value);},None=>{command.env_remove("FONTCONFIG_FILE");}}}
/// Steam/Wine/Proton retain the user's original font environment.
pub fn child_command(binary:impl AsRef<std::ffi::OsStr>)->Command{
    let mut command=Command::new(binary);if let Some(original)=ORIGINAL.get(){restore(&mut command,original.as_ref());}crate::portable::host_environment(&mut command);command
}
#[cfg(test)] mod tests{
    use super::*;
    #[test] fn private_config_has_only_bundled_fonts_and_private_cache(){
        let runtime=prepare(&std::env::temp_dir()).unwrap();let config=fs::read_to_string(runtime.0.join("fonts.conf")).unwrap();
        assert!(!config.contains("<include>"));assert!(config.contains("/cache</cachedir>"));assert!(runtime.0.join("fonts/DejaVuSans-Bold.ttf").is_file());
        let path=runtime.0.clone();drop(runtime);assert!(!path.exists());
    }
    #[test] fn xml_paths_are_escaped(){assert_eq!(xml("a&<\"'>"),"a&amp;&lt;&quot;&apos;&gt;");}
    #[test] fn game_commands_restore_or_remove_the_override(){
        let mut c=Command::new("fixture");let value=OsString::from("/user/fonts.conf");restore(&mut c,Some(&value));
        assert!(c.get_envs().any(|(k,v)|k=="FONTCONFIG_FILE" && v==Some(value.as_os_str())));
        restore(&mut c,None);assert!(c.get_envs().any(|(k,v)|k=="FONTCONFIG_FILE" && v.is_none()));
    }
    #[test] fn fontconfig_reads_private_fonts_without_a_cache_rebuild(){
        let runtime=prepare(&std::env::temp_dir()).unwrap();
        let result=Command::new("fc-list").env("FONTCONFIG_FILE",runtime.0.join("fonts.conf")).args(["--format","%{file}\\n"]).output().expect("fontconfig test prerequisite: fc-list");
        assert!(result.status.success());let output=String::from_utf8(result.stdout).unwrap();
        assert_eq!(output.lines().count(),2);assert!(output.lines().all(|line|line.starts_with(runtime.0.to_str().unwrap())));
    }
}
