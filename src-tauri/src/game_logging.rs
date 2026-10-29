//! Targeted, backed-up Steam LaunchOptions edits. No shell execution.
use crate::{auto_select::{self, Token}, repairs, scanner};
use serde::Serialize;
use std::{collections::HashMap, fs, path::{Path, PathBuf}, sync::{LazyLock, Mutex}};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Status { pub state: String, pub message: String, pub backup_path: Option<String>, pub can_restore: bool }
struct Record { _activity: Option<std::fs::File>, target: PathBuf, status: Status, before: Option<String>, after: Option<String> }
static REQUESTS: LazyLock<Mutex<HashMap<(String,String),Record>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) fn require_update_idle()->Result<(),String>{
    let records=REQUESTS.try_lock().map_err(|_|"Wait for game logging setup")?;
    if records.values().any(|r|r.status.state=="waiting-for-steam") {
        return Err("Close Steam and finish queued game logging setup before closing or updating.".into());
    }
    Ok(())
}

fn options(original: &str) -> Result<String,String> {
    if original.len()>8192 || original.chars().any(|c| matches!(c,';'|'|'|'&'|'`'|'\n'|'\r'|'<'|'>'|'$'|'('|')')) {
        return Err("Complex launch script preserved unchanged. Automatic log setup cannot safely rewrite it.".into());
    }
    if original.contains("PROTON_LOG_DIR") { return Err("Custom log destination preserved unchanged; automatic setup for custom destinations is not supported yet.".into()); }
    let count=original.matches("%command%").count();
    if count>1 { return Err("Multiple game command placeholders; launch options were not changed".into()); }
    let mut spans=Vec::new();let mut start=None;let mut quote=None;
    for (i,c) in original.char_indices() {
        if c=='\\' { return Err("Escaped launch script preserved unchanged; review its options manually".into()); }
        if c=='\'' || c=='"' { if quote==Some(c) {quote=None;} else if quote.is_none(){quote=Some(c);} }
        if c.is_whitespace() && quote.is_none() { if let Some(a)=start.take(){spans.push((a,i));} }
        else if start.is_none(){start=Some(i);}
    }
    if quote.is_some(){return Err("Unbalanced launch-option quotes".into());}
    if let Some(a)=start {spans.push((a,original.len()));}
    let log:Vec<_>=spans.iter().copied().filter(|(a,b)|original[*a..*b].starts_with("PROTON_LOG=")).collect();
    if original.contains("PROTON_LOG") && log.len()!=1 {return Err("Ambiguous Proton logging assignment; existing options preserved".into());}
    if let Some(&(a,b))=log.first() {
        if count!=1 || a>original.find("%command%").unwrap_or(0) {return Err("Logging assignment is not before the game command; existing options preserved".into());}
        let mut result=original.to_owned();result.replace_range(a..b,"PROTON_LOG=1");return Ok(result);
    }
    if count==1 { Ok(format!("PROTON_LOG=1 {original}")) }
    else if original.trim().is_empty(){Ok("PROTON_LOG=1 %command%".into())}
    else {Ok(format!("PROTON_LOG=1 %command% {original}"))}
}
fn quote(value: &str) -> String { format!("\"{}\"",value.replace('\\',"\\\\").replace('"',"\\\"").replace('\n',"\\n").replace('\t',"\\t")) }
fn children(t:&[Token], open:Option<usize>) -> Result<Vec<(usize,usize)>,String> {
    let end=match open {Some(i)=>auto_select::matching_close(t,i)?,None=>t.len()};
    let mut i=open.map_or(0,|n|n+1);let mut out=vec![];
    while i<end {
        let v=t.get(i+1).ok_or("Malformed local Steam config")?;
        let close=if !v.quoted && v.value=="{" {auto_select::matching_close(t,i+1)?}else{i+1};
        out.push((i,close));i=close+1;
    }
    Ok(out)
}
fn child(t:&[Token],open:Option<usize>,key:&str)->Result<Option<(usize,usize)>,String>{
    let found:Vec<_>=children(t,open)?.into_iter().filter(|(i,_)|t[*i].value.eq_ignore_ascii_case(key)).collect();
    if found.len()>1{return Err(format!("Ambiguous {key} in Steam config"));}Ok(found.first().copied())
}
fn locate(text:&str,app:&str)->Result<(Vec<Token>,usize,Option<(usize,usize)>),String>{
    if app.is_empty() || !app.bytes().all(|b|b.is_ascii_digit()){return Err("Invalid game ID".into());}
    scanner::parse_vdf(text)?;let t=auto_select::tokens(text)?;let mut open=None;
    for key in ["UserLocalConfigStore","Software","Valve","Steam"] {
        let (key_i,_)=child(&t,open,key)?.ok_or("Unrecognized local Steam configuration layout")?;
        if t[key_i+1].quoted || t[key_i+1].value!="{" {return Err("Expected Steam configuration object".into());}
        open=Some(key_i+1);
    }
    let steam=open.unwrap();let apps=child(&t,Some(steam),"apps")?;
    Ok((t,steam,apps))
}
fn launch_value(text:&str,app:&str)->Result<Option<String>,String>{
    let(t,_,apps)=locate(text,app)?;let Some((apps,_))=apps else{return Ok(None)};
    if t[apps+1].quoted || t[apps+1].value!="{" {return Err("Invalid apps object".into());}
    let Some((game,_))=child(&t,Some(apps+1),app)? else{return Ok(None)};
    if t[game+1].quoted || t[game+1].value!="{" {return Err("Invalid game object".into());}
    let Some((key,_))=child(&t,Some(game+1),"LaunchOptions")? else{return Ok(None)};
    if !t[key+1].quoted {return Err("Invalid launch options value".into());}
    Ok(Some(t[key+1].value.clone()))
}
fn set_launch_value(text:&str,app:&str,value:Option<&str>)->Result<String,String>{
    let(t,steam,apps)=locate(text,app)?;let mut result=text.to_string();
    let field=||format!("\n\"LaunchOptions\" {}\n",quote(value.unwrap_or("")));
    if let Some((apps,_))=apps {
        if t[apps+1].quoted || t[apps+1].value!="{" {return Err("Invalid apps object".into());}
        if let Some((game,end))=child(&t,Some(apps+1),app)? {
            if t[game+1].quoted || t[game+1].value!="{" {return Err("Invalid game object".into());}
            if let Some((key,last))=child(&t,Some(game+1),"LaunchOptions")? {
                if !t[key+1].quoted{return Err("Invalid launch options value".into());}
                if let Some(value)=value {result.replace_range(t[key+1].start..t[key+1].end,&quote(value));}
                else {result.replace_range(t[key].start..t[last].end,"");}
            } else if value.is_some(){result.insert_str(t[end].start,&field());}
        } else if value.is_some(){let end=auto_select::matching_close(&t,apps+1)?;result.insert_str(t[end].start,&format!("\n\"{app}\" {{ {} }}\n",field()));}
    } else if value.is_some(){let end=auto_select::matching_close(&t,steam)?;result.insert_str(t[end].start,&format!("\n\"apps\" {{ \"{app}\" {{ {} }} }}\n",field()));}
    scanner::parse_vdf(&result)?;Ok(result)
}
fn regular_chain(root:&Path,target:&Path)->Result<(),String>{
    let relative=target.strip_prefix(root).map_err(|_|"Config outside Steam installation")?;
    let mut p=root.to_path_buf();
    for part in relative.components(){p.push(part);if fs::symlink_metadata(&p).map_err(|e|e.to_string())?.file_type().is_symlink(){return Err("Symlinked Steam user configuration is not edited automatically".into());}}
    auto_select::check_regular(target)
}
fn account_config(root:&Path)->Result<PathBuf,String>{
    let login=root.join("config/loginusers.vdf");regular_chain(root,&login)?;
    let parsed=scanner::load_vdf(&login)?;let users=scanner::obj_get(&parsed,"users").ok_or("No Steam user records")?;
    let mut accounts=vec![];
    for (id,value) in users {
        if let scanner::Value::Object(props)=value {
            if scanner::text_get(props,"MostRecent")==Some("1") {
                let id=id.parse::<u64>().ok().and_then(|i|i.checked_sub(76561197960265728)).filter(|i|*i<=u32::MAX as u64).ok_or("Invalid Steam user ID")?;
                accounts.push(id);
            }
        }
    }
    if accounts.len()!=1{return Err("Cannot identify one most-recent Steam account. Sign into the intended account, close Steam, then diagnose again.".into());}
    let target=root.join("userdata").join(accounts[0].to_string()).join("config/localconfig.vdf");regular_chain(root,&target)?;Ok(target)
}
fn target(app:&str,library:&str)->Result<PathBuf,String>{
    let scan=scanner::scan();
    if !scan.games.iter().any(|g|g.app_id==app && g.library_path==library){return Err("Game not found in current Steam scan".into());}
    let config=auto_select::config_for_library(&scan,library)?;
    let root=config.parent().and_then(Path::parent).ok_or("Missing Steam installation")?;
    account_config(root)
}
fn enable_file(path:&Path,app:&str,guard:bool)->Result<(Status,Option<String>,Option<String>),String>{
    auto_select::check_regular(path)?;let bytes=fs::read(path).map_err(|e|e.to_string())?;
    let text=std::str::from_utf8(&bytes).map_err(|e|e.to_string())?;
    let before=launch_value(text,app)?;let after=options(before.as_deref().unwrap_or(""))?;
    if before.as_deref()==Some(after.as_str()){return Ok((Status{state:"ready".into(),message:"Proton logging is already enabled for this game.".into(),backup_path:None,can_restore:false},before,Some(after)));}
    if guard && repairs::require_steam_closed().is_err(){return Ok((Status{state:"waiting-for-steam".into(),message:"Log setup queued. Close Steam completely; Proton Medic will enable this game's logging automatically.".into(),backup_path:None,can_restore:false},None,None));}
    let updated=set_launch_value(text,app,Some(&after))?;
    let(backup,_)=auto_select::write_config(path,&bytes,updated.as_bytes(),guard)?;
    Ok((Status{state:"ready".into(),message:"Proton logging enabled. Launch the game to collect a new diagnostic log.".into(),backup_path:Some(backup),can_restore:true},before,Some(after)))
}
pub fn prepare(app:String,library:String,confirmed:bool)->Result<Status,String>{
    if !confirmed{return Err("Approve diagnosing this game before enabling logs".into());}
    let activity=crate::coordination::activity()?;
    let mut records=REQUESTS.lock().map_err(|_|"Log setup lock unavailable")?;
    let key=(app.clone(),library.clone());
    if let Some(record)=records.get(&key){if record.status.state=="waiting-for-steam" && repairs::require_steam_closed().is_err(){return Ok(record.status.clone());}}
    if records.len()>=64 && !records.contains_key(&key){return Err("Too many logging setup requests in this app session".into());}
    let _guard=auto_select::CONFIG_LOCK.lock().map_err(|_|"Config lock unavailable")?;
    let path=target(&app,&library)?;
    if let Some(old)=records.get(&key){if old.status.state=="waiting-for-steam" && old.target!=path{return Err("Steam account changed while queued. Close and reopen Proton Medic before diagnosing the new account.".into());}}
    let(status,before,after)=enable_file(&path,&app,true)?;
    // Repeated diagnosis must not discard the original restore receipt.
    if status.backup_path.is_none() && status.state=="ready" {
        if let Some(old)=records.get(&key){if old.target==path && old.after==after && old.status.can_restore{return Ok(old.status.clone());}}
    }
    records.insert(key,Record{_activity:if status.state=="waiting-for-steam"{Some(activity)}else{None},target:path,status:status.clone(),before,after});Ok(status)
}
pub fn process_pending(){
    let Ok(mut records)=REQUESTS.try_lock() else{return};
    if !records.values().any(|r|r.status.state=="waiting-for-steam"){return;}
    if repairs::require_steam_closed().is_err(){return;}
    for ((app,library),record) in records.iter_mut().filter(|(_,r)|r.status.state=="waiting-for-steam").take(1){
        let result=(||{
            let _guard=auto_select::CONFIG_LOCK.lock().map_err(|_|"Config lock unavailable")?;
            if target(app,library)?!=record.target{return Err("Steam account changed while log setup was queued. Diagnose again.".into());}
            enable_file(&record.target,app,true)
        })();
        match result{Ok((s,b,a))=>{record.status=s;record.before=b;record.after=a;},Err(e)=>{record.status.state="needs-attention".into();record.status.message=e;}}
        if record.status.state!="waiting-for-steam" { record._activity=None; }
    }
}
pub fn status(app:&str,library:&str)->Result<Option<Status>,String>{Ok(REQUESTS.lock().map_err(|_|"Log setup lock unavailable")?.get(&(app.into(),library.into())).map(|r|r.status.clone()))}
pub fn restore(app:String,library:String,confirmed:bool)->Result<Status,String>{
    if !confirmed{return Err("Approve restoring this game's launch options".into());}
    let mut records=REQUESTS.lock().map_err(|_|"Log setup lock unavailable")?;
    let record=records.get_mut(&(app.clone(),library.clone())).ok_or("Logging receipt unavailable; original backup remains on disk")?;
    if !record.status.can_restore{return Err("No logging change to restore in this session".into());}
    let _guard=auto_select::CONFIG_LOCK.lock().map_err(|_|"Config lock unavailable")?;
    repairs::require_steam_closed()?;
    if target(&app,&library)?!=record.target{return Err("Steam account changed; refusing to restore a different account".into());}
    let backup=restore_file(&record.target,&app,record.before.as_deref(),record.after.as_deref(),true)?;
    record.status=Status{state:"restored".into(),message:"Previous launch options restored; other Steam settings preserved.".into(),backup_path:Some(backup),can_restore:false};Ok(record.status.clone())
}
fn restore_file(path:&Path,app:&str,before:Option<&str>,after:Option<&str>,guard:bool)->Result<String,String>{
    auto_select::check_regular(path)?;let bytes=fs::read(path).map_err(|e|e.to_string())?;
    let text=std::str::from_utf8(&bytes).map_err(|e|e.to_string())?;
    if launch_value(text,app)?.as_deref()!=after{return Err("Launch options changed after log setup; refusing to overwrite newer settings".into());}
    let updated=set_launch_value(text,app,before)?;
    auto_select::write_config(path,&bytes,updated.as_bytes(),guard).map(|(backup,_)|backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    const CONFIG:&str=r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" {
      "Unrelated" "old" // retain comment
      "apps" { "123" { "LaunchOptions" "-dx11 -windowed" "Other" "keep" } "456" { "LaunchOptions" "-safe" } }
    } } } }"#;
    struct Fixture(PathBuf);
    impl Fixture {fn new()->Self{let p=std::env::temp_dir().join(format!("pm-logging-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&p).unwrap();Self(p)}}
    impl Drop for Fixture{fn drop(&mut self){let _=fs::remove_dir_all(&self.0);}}
    #[test] fn enables_blank_and_argument_only_options(){
        assert_eq!(options("").unwrap(),"PROTON_LOG=1 %command%");
        assert_eq!(options("-dx11 -windowed").unwrap(),"PROTON_LOG=1 %command% -dx11 -windowed");
    }
    #[test] fn preserves_wrapper_and_replaces_existing_log_assignment(){
        assert_eq!(options("gamemoderun %command% -dx11").unwrap(),"PROTON_LOG=1 gamemoderun %command% -dx11");
        assert_eq!(options("DXVK_ASYNC=1 PROTON_LOG=0 %command% -dx11").unwrap(),"DXVK_ASYNC=1 PROTON_LOG=1 %command% -dx11");
        assert_eq!(options("PROTON_LOG=1 %command%").unwrap(),"PROTON_LOG=1 %command%");
    }
    #[test] fn refuses_ambiguous_or_complex_scripts(){
        for s in ["echo a; %command%", "PROTON_LOG=1 PROTON_LOG=0 %command%", "%command% %command%", "env -u PROTON_LOG %command%", "PROTON_LOG_DIR=/custom %command%", "sh -c 'exec $0' %command%"] {assert!(options(s).is_err(),"{s}");}
    }
    #[test] fn edits_only_target_leaf_and_preserves_comments(){
        let value=options(launch_value(CONFIG,"123").unwrap().as_deref().unwrap()).unwrap();
        let updated=set_launch_value(CONFIG,"123",Some(&value)).unwrap();
        assert_eq!(launch_value(&updated,"123").unwrap().as_deref(),Some(value.as_str()));
        assert_eq!(launch_value(&updated,"456").unwrap().as_deref(),Some("-safe"));
        assert!(updated.contains("// retain comment"));assert!(updated.contains("\"Other\" \"keep\""));
    }
    #[test] fn creates_missing_game_and_missing_apps_only_in_known_layout(){
        let bare=r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" {} } } }"#;
        for text in [bare,CONFIG] {
            let result=set_launch_value(text,"999",Some("PROTON_LOG=1 %command%")).unwrap();
            assert_eq!(launch_value(&result,"999").unwrap().as_deref(),Some("PROTON_LOG=1 %command%"));
        }
        assert!(set_launch_value("\"Other\" {}","123",Some("x")).is_err());
        assert!(set_launch_value(CONFIG,"../bad",Some("x")).is_err());
    }
    #[test] fn rejects_duplicate_target_keys(){
        let bad=CONFIG.replace("\"LaunchOptions\" \"-dx11 -windowed\"","\"LaunchOptions\" \"a\" \"LaunchOptions\" \"b\"");
        assert!(launch_value(&bad,"123").is_err());
        assert!(set_launch_value(&bad,"123",Some("x")).is_err());
    }
    #[test] fn backup_enable_and_selective_restore_are_real_file_operations(){
        let f=Fixture::new();let path=f.0.join("localconfig.vdf");fs::write(&path,CONFIG).unwrap();
        let(status,before,after)=enable_file(&path,"123",false).unwrap();assert!(status.can_restore);
        assert_eq!(fs::read_to_string(status.backup_path.unwrap()).unwrap(),CONFIG);
        let current=fs::read_to_string(&path).unwrap().replace("\"Unrelated\" \"old\"","\"Unrelated\" \"new\"");fs::write(&path,&current).unwrap();
        let backup=restore_file(&path,"123",before.as_deref(),after.as_deref(),false).unwrap();
        assert_eq!(fs::read_to_string(backup).unwrap(),current);
        let restored=fs::read_to_string(&path).unwrap();assert!(restored.contains("\"Unrelated\" \"new\""));assert_eq!(launch_value(&restored,"123").unwrap(),before);
    }
    #[test] fn idempotent_setup_makes_no_second_backup_and_restore_rejects_new_options(){
        let f=Fixture::new();let path=f.0.join("localconfig.vdf");fs::write(&path,CONFIG).unwrap();
        let(_,before,after)=enable_file(&path,"123",false).unwrap();
        let(second,_,_)=enable_file(&path,"123",false).unwrap();assert!(second.backup_path.is_none());
        assert_eq!(fs::read_dir(&f.0).unwrap().count(),2);
        let user=set_launch_value(&fs::read_to_string(&path).unwrap(),"123",Some("-user-change")).unwrap();fs::write(&path,&user).unwrap();
        assert!(restore_file(&path,"123",before.as_deref(),after.as_deref(),false).is_err());assert_eq!(fs::read_to_string(&path).unwrap(),user);
    }
    #[test] fn restore_missing_original_launch_options_removes_only_added_field(){
        let f=Fixture::new();let path=f.0.join("localconfig.vdf");let original=set_launch_value(CONFIG,"123",None).unwrap();fs::write(&path,&original).unwrap();
        let(_,before,after)=enable_file(&path,"123",false).unwrap();assert!(before.is_none());restore_file(&path,"123",None,after.as_deref(),false).unwrap();
        let restored=fs::read_to_string(path).unwrap();assert!(launch_value(&restored,"123").unwrap().is_none());assert!(restored.contains("\"Other\" \"keep\""));
    }
    #[test] fn resolves_most_recent_account_and_refuses_ambiguity_or_symlinks(){
        let f=Fixture::new();fs::create_dir_all(f.0.join("config")).unwrap();fs::create_dir_all(f.0.join("userdata/123/config")).unwrap();
        let login=f.0.join("config/loginusers.vdf");fs::write(&login,"\"users\" { \"76561197960265851\" { \"MostRecent\" \"1\" } }").unwrap();
        let local=f.0.join("userdata/123/config/localconfig.vdf");fs::write(&local,CONFIG).unwrap();assert_eq!(account_config(&f.0).unwrap(),local);
        fs::write(&login,"\"users\" { \"76561197960265851\" { \"MostRecent\" \"1\" } \"76561197960265852\" { \"MostRecent\" \"1\" } }").unwrap();assert!(account_config(&f.0).is_err());
        fs::remove_file(&login).unwrap();std::os::unix::fs::symlink(&local,&login).unwrap();assert!(account_config(&f.0).is_err());
    }
    #[test] fn rejects_file_symlinks_and_requires_approval(){
        let f=Fixture::new();let actual=f.0.join("real");fs::write(&actual,CONFIG).unwrap();let link=f.0.join("localconfig.vdf");std::os::unix::fs::symlink(&actual,&link).unwrap();
        assert!(enable_file(&link,"123",false).is_err());assert_eq!(fs::read_to_string(actual).unwrap(),CONFIG);
        assert!(prepare("123".into(),"/none".into(),false).unwrap_err().contains("Approve"));
        assert!(restore("123".into(),"/none".into(),false).unwrap_err().contains("Approve"));
    }
}
