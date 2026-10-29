//! Per-game Proton auto-selection. Never edits active Steam configuration.
//! Edits are minimal, atomic, backed up, and restricted to a scanned game.
use crate::{advisor, diagnostics, repairs, scanner};
use serde::Serialize;
use std::{collections::HashSet, fs, io::Write, os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf}, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

pub(crate) static CONFIG_LOCK: Mutex<()> = Mutex::new(());
const MAX_CONFIG: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct AutoSelection {
    pub game_name: String,
    pub app_id: String,
    pub tool_name: String,
    pub tool_id: String,
    pub message: String,
    pub backup_path: String,
    pub changed: bool,
}

// Token offsets permit a single leaf edit without normalizing or deleting
// comments, whitespace, or fields Steam doesn't expose to Proton Medic.
#[derive(Debug)]
pub(crate) struct Token { pub value: String, pub start: usize, pub end: usize, pub quoted: bool }

pub(crate) fn tokens(s: &str) -> Result<Vec<Token>, String> {
    let bytes=s.as_bytes(); let mut i=0; let mut output=Vec::new();
    while i<bytes.len() {
        if bytes[i].is_ascii_whitespace() { i+=1; continue; }
        if bytes[i]==b'/' && i+1<bytes.len() && bytes[i+1]==b'/' {
            i+=2; while i<bytes.len() && bytes[i]!=b'\n' {i+=1;} continue;
        }
        if bytes[i]==b'/' && i+1<bytes.len() && bytes[i+1]==b'*' {
            i+=2; while i+1<bytes.len() && &bytes[i..i+2]!=b"*/" { i+=1; }
            if i+1>=bytes.len() { return Err("Unterminated VDF comment".into()); }
            i+=2; continue;
        }
        let start=i;
        if bytes[i]==b'{' || bytes[i]==b'}' {
            i+=1; output.push(Token{value:s[start..i].into(),start,end:i,quoted:false});
        } else if bytes[i]==b'"' {
            i+=1; let mut value=String::new(); let mut closed=false;
            while i<bytes.len() {
                if bytes[i]==b'"' {i+=1; closed=true; break;}
                if bytes[i]==b'\\' && i+1<bytes.len() && matches!(bytes[i+1],b'"'|b'\\') {
                    value.push(bytes[i+1] as char);i+=2;continue;
                }
                let c=s[i..].chars().next().ok_or("Invalid text encoding")?;
                value.push(c);i+=c.len_utf8();
            }
            if !closed {return Err("Unterminated VDF string".into());}
            output.push(Token{value,start,end:i,quoted:true});
        } else {
            while i<bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i]!=b'{' && bytes[i]!=b'}' {i+=1;}
            output.push(Token{value:s[start..i].into(),start,end:i,quoted:false});
        }
        if output.len()>200_000 { return Err("Steam config has too many tokens".into()); }
    }
    Ok(output)
}

pub(crate) fn matching_close(t: &[Token], opening: usize) -> Result<usize,String> {
    if t.get(opening).map(|x| (!x.quoted,x.value.as_str()))!=Some((true,"{")) {return Err("Expected opening brace".into());}
    let mut depth=0;
    for (i, tok) in t.iter().enumerate().skip(opening) {
        if !tok.quoted && tok.value=="{" {depth+=1;}
        else if !tok.quoted && tok.value=="}" {depth-=1; if depth==0 {return Ok(i);} }
    }
    Err("Unbalanced VDF object".into())
}

fn replacement_text(original: &str, app_id: &str, tool_id: &str) -> Result<(String, bool),String> {
    if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) {return Err("Invalid Steam App ID".into());}
    if tool_id.is_empty() || tool_id.len()>128 || !tool_id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b'_'|b'-'|b'.')) {
        return Err("Unsafe Proton compatibility identifier".into());
    }
    // The existing configuration must be parseable. Refuse partially corrupt configs.
    scanner::parse_vdf(original)?;
    let all=tokens(original)?;
    let locations:Vec<usize>=all.iter().enumerate().filter_map(|(i,t)|
        (t.quoted && t.value.eq_ignore_ascii_case("CompatToolMapping") && all.get(i+1).is_some_and(|n| !n.quoted && n.value=="{"))
            .then_some(i+1)).collect();
    if locations.len()>1 {return Err("Steam compatibility mapping is ambiguous".into());}
    if locations.is_empty() {
        // A fresh Steam configuration may not have a CompatToolMapping yet.
        // Only insert into a uniquely identifiable Steam object under an
        // InstallConfigStore root, rather than modifying an arbitrary object.
        if !all.iter().any(|t|t.quoted && t.value=="InstallConfigStore") {
            return Err("Unrecognized Steam config layout; open Steam Settings → Compatibility first".into());
        }
        let steam_blocks:Vec<usize>=all.iter().enumerate().filter_map(|(i,t)|
            (t.quoted && t.value=="Steam" && all.get(i+1).is_some_and(|n|!n.quoted && n.value=="{"))
                .then_some(i+1)).collect();
        if steam_blocks.len()!=1 {return Err("Steam config object is missing or ambiguous".into());}
        let steam_end=matching_close(&all,steam_blocks[0])?;
        let mut out=original.to_string();
        out.insert_str(all[steam_end].start,&format!("\n\t\t\t\t\"CompatToolMapping\" {{ \"{app_id}\" {{ \"name\" \"{tool_id}\" \"config\" \"\" \"priority\" \"250\" }} }}\n"));
        scanner::parse_vdf(&out)?;
        return Ok((out,true));
    }
    let open=locations[0]; let end=matching_close(&all,open)?;
    let mut i=open+1;
    while i<end {
        let key=&all[i];
        let Some(value)=all.get(i+1) else {return Err("Malformed compatibility mapping".into())};
        if key.value==app_id {
            if value.quoted || value.value!="{" {return Err("Unexpected game compatibility mapping value".into());}
            let item_end=matching_close(&all,i+1)?;
            let mut j=i+2;
            while j<item_end {
                let item=&all[j];
                if item.value.eq_ignore_ascii_case("name") {
                    let v=all.get(j+1).ok_or("Missing name value")?;
                    if !v.quoted || v.value=="{" || v.value=="}" {return Err("Invalid compatibility name".into());}
                    if v.value==tool_id {return Ok((original.into(),false));}
                    let mut out=original.to_string();out.replace_range(v.start..v.end,&format!("\"{tool_id}\""));
                    scanner::parse_vdf(&out)?;
                    return Ok((out,true));
                }
                j+=1;
            }
            // Mapping exists, but without a name key.
            let mut out=original.to_string();out.insert_str(all[item_end].start,&format!("\n\t\t\t\"name\" \"{tool_id}\"\n"));
            scanner::parse_vdf(&out)?;return Ok((out,true));
        }
        i=if !value.quoted && value.value=="{" {matching_close(&all,i+1)?+1} else {i+2};
    }
    let mut out=original.to_string();
    out.insert_str(all[end].start,&format!("\n\t\t\"{app_id}\" {{ \"name\" \"{tool_id}\" \"config\" \"\" \"priority\" \"250\" }}\n"));
    scanner::parse_vdf(&out)?;
    Ok((out,true))
}

pub(crate) fn check_regular(path:&Path)->Result<(),String>{
    let meta=fs::symlink_metadata(path).map_err(|e|format!("Cannot inspect Steam config: {e}"))?;
    if !meta.is_file() || meta.file_type().is_symlink() {return Err("Steam configuration must be a regular file, not a symlink".into());}
    if meta.len()>MAX_CONFIG {return Err("Steam configuration exceeds the safety size limit".into());}
    Ok(())
}
fn unix_stamp()->Result<u128,String>{Ok(SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_nanos())}

fn replace_config(path:&Path,app_id:&str,tool_id:&str,guard_steam:bool)->Result<(String,bool),String>{
    check_regular(path)?;
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    let content=std::str::from_utf8(&bytes).map_err(|_|"Steam config is not valid UTF-8; refuse to rewrite it")?;
    let (updated,changed)=replacement_text(content,app_id,tool_id)?;
    if !changed {return Ok((String::new(),false));}
    write_config(path, &bytes, updated.as_bytes(), guard_steam)
}

pub(crate) fn write_config(path: &Path, bytes: &[u8], updated: &[u8], guard_steam: bool) -> Result<(String,bool),String> {
    let _lease = crate::coordination::config(path)?;
    check_regular(path)?;
    if fs::read(path).map_err(|e|e.to_string())? != bytes {
        return Err("Steam configuration changed during the operation; original was not overwritten".into());
    }
    let parent=path.parent().ok_or("Steam config has no parent folder")?;
    let file_name=path.file_name().and_then(|s|s.to_str()).ok_or("Invalid config filename")?;
    let stamp=unix_stamp()?;
    let backup=parent.join(format!("{file_name}.proton-medic-{stamp}.bak"));
    let tmp=parent.join(format!(".{file_name}.proton-medic-{stamp}.tmp"));
    let perms=fs::metadata(path).map_err(|e|e.to_string())?.permissions();
    // Create immutable backup BEFORE making any changes. No overwrites.
    fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&backup)
        .and_then(|mut f|{f.write_all(bytes)?;f.sync_all()})
        .map_err(|e|format!("Cannot save Steam configuration backup: {e}"))?;
    let result=(||->Result<(),String>{
        let mut f=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&tmp).map_err(|e|e.to_string())?;
        f.write_all(updated).map_err(|e|e.to_string())?;
        f.set_permissions(perms).map_err(|e|e.to_string())?;
        f.sync_all().map_err(|e|e.to_string())?;
        // Fail if Steam started during this operation or config changed while examined.
        if guard_steam { repairs::require_steam_closed()?; }
        check_regular(path)?;
        if fs::read(path).map_err(|e|e.to_string())?!=bytes {return Err("Steam configuration changed during the operation; original was not overwritten".into());}
        fs::rename(&tmp,path).map_err(|e|format!("Cannot atomically replace config: {e}"))?;
        let _=fs::File::open(parent).and_then(|f|f.sync_all());
        Ok(())
    })();
    if result.is_err(){let _=fs::remove_file(&tmp);}
    result?;
    Ok((backup.to_string_lossy().to_string(),true))
}

// Use the custom tool's manifest name, not its directory/display name.
// Guessing this ID can leave a game unlaunchable in Steam.
pub(crate) fn internal_id(name:&str,path:&Path)->Result<String,String>{
    let file=path.join("compatibilitytool.vdf");
    if file.is_file() {
        let parsed=scanner::load_vdf(&file)?;
        let container=scanner::obj_get(&parsed,"compatibilitytools").unwrap_or(&parsed);
        let tools=scanner::obj_get(container,"compat_tools").ok_or("Missing compat_tools in tool manifest")?;
        let names:Vec<&String>=tools.iter().filter_map(|(name,value)|
            matches!(value,scanner::Value::Object(_)).then_some(name)).collect();
        if names.len()!=1 {return Err("Cannot identify unique compatibility tool ID".into());}
        return Ok(names[0].clone());
    }
    let lower=name.to_ascii_lowercase();
    if lower=="proton experimental" {return Ok("proton_experimental".into());}
    if lower=="proton hotfix" {return Ok("proton_hotfix".into());}
    if let Some(suffix)=lower.strip_prefix("proton ") {
        if let Some(major)=suffix.split('.').next() {
            if !major.is_empty() && major.bytes().all(|b|b.is_ascii_digit()) {
                return Ok(format!("proton_{major}"));
            }
        }
    }
    Err("No verified Steam compatibility identifier for this tool".into())
}

pub(crate) fn config_for_library(scan:&scanner::ScanReport,library:&str)->Result<PathBuf,String>{
    let canon=fs::canonicalize(library).map_err(|e|format!("Steam library is unavailable: {e}"))?;
    let mut choices=HashSet::new();
    for install in &scan.installations {
        let root=Path::new(&install.path);
        for lib in scanner::libraries_from_vdf(root,&mut Vec::new()) {
            if fs::canonicalize(lib).ok().as_deref()==Some(canon.as_path()) {
                let config=root.join("config/config.vdf");
                if config.is_file() {choices.insert(config);}
            }
        }
    }
    if choices.len()!=1 {return Err("Could not identify one Steam installation for this game; select the game from the correct library".into());}
    let path=choices.into_iter().next().ok_or("No Steam configuration found")?;
    let config_dir=path.parent().ok_or("Steam config directory unavailable")?;
    if fs::symlink_metadata(config_dir).map_err(|e|e.to_string())?.file_type().is_symlink() {
        return Err("Steam config directory is a symbolic link; automatic editing is disabled".into());
    }
    Ok(path)
}

pub fn select(app_id:String,library_path:String,confirmed:bool)->Result<AutoSelection,String>{
    select_tracked(app_id, library_path, confirmed, None).map(|(result, _)| result)
}

fn allowed_tool_roots(config: &Path) -> Result<Vec<PathBuf>, String> {
    let steam = config.parent().and_then(Path::parent).ok_or("Steam root unavailable")?;
    let mut roots = vec![steam.join("compatibilitytools.d")];
    roots.extend(scanner::libraries_from_vdf(steam, &mut vec![]).into_iter().map(|p| p.join("steamapps/common")));
    Ok(roots.into_iter().filter_map(|p| fs::canonicalize(p).ok()).collect())
}

fn tool_allowed(tool: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| tool.starts_with(root))
}

pub fn manual_tools(app_id: &str, library_path: &str) -> Result<Vec<scanner::CompatibilityTool>, String> {
    let scan = crate::inventory::steam();
    if !scan.games.iter().any(|g| g.app_id == app_id && g.library_path == library_path) {
        return Err("Game is no longer in this Steam library".into());
    }
    tools_for_library(&scan, library_path)
}

pub(crate) fn tools_for_library(scan: &scanner::ScanReport, library_path: &str) -> Result<Vec<scanner::CompatibilityTool>, String> {
    let roots = allowed_tool_roots(&config_for_library(scan, library_path)?)?;
    Ok(scan.compatibility_tools.iter().filter(|tool| fs::canonicalize(&tool.path).is_ok_and(|path| tool_allowed(&path, &roots))).cloned().collect())
}

pub fn select_manual(app_id:String,library_path:String,tool_path:String,confirmed:bool)->Result<AutoSelection,String>{
    if !confirmed{return Err("Approve changing this game's Proton version".into());}
    let _lock=CONFIG_LOCK.lock().map_err(|_|"Configuration lock unavailable")?;
    repairs::require_steam_closed()?;
    let scan=scanner::scan();
    let game=scan.games.iter().find(|g|g.app_id==app_id&&g.library_path==library_path).ok_or("Game is no longer in this Steam library")?;
    let selected=scan.compatibility_tools.iter().find(|t|t.path==tool_path).ok_or("Selected Proton is no longer installed")?;
    let config=config_for_library(&scan,&library_path)?;
    let tool=fs::canonicalize(&tool_path).map_err(|e|e.to_string())?;
    if !tool_allowed(&tool, &allowed_tool_roots(&config)?) { return Err("Choose a Proton installed for this game's Steam installation".into()); }
    let id=internal_id(&selected.name,&tool)?;
    let (backup_path,changed)=replace_config(&config,&app_id,&id,true)?;
    crate::inventory::refresh();
    Ok(AutoSelection{game_name:game.name.clone(),app_id,tool_name:selected.name.clone(),tool_id:id,backup_path,changed,
        message:if changed{"Proton selected for this game. Previous configuration backed up. Open Steam and test the game."}else{"This Proton is already selected. No configuration changes were needed."}.into()})
}

pub(crate) struct ConfigUndo { path: PathBuf, app_id: String, before: Vec<u8>, after: Vec<u8> }

pub(crate) fn rollback(undo: &ConfigUndo, confirmed: bool) -> Result<String, String> {
    if !confirmed { return Err("Explicit rollback approval required".into()); }
    let _lock = CONFIG_LOCK.lock().map_err(|_| "Configuration lock unavailable")?;
    repairs::require_steam_closed()?;
    rollback_file(undo, true)
}

fn rollback_file(undo: &ConfigUndo, guard_steam: bool) -> Result<String, String> {
    check_regular(&undo.path)?;
    let current = fs::read(&undo.path).map_err(|e| e.to_string())?;
    let restored = rollback_text(undo, &current)?;
    write_config(&undo.path, &current, restored.as_bytes(), guard_steam).map(|(backup, _)| backup)
}

// Restore only this game's mapping. Unrelated Steam changes are retained.
fn mapping_range(content: &str, app_id: &str) -> Result<Option<(usize, usize)>, String> {
    scanner::parse_vdf(content)?;
    let t = tokens(content)?;
    let blocks: Vec<usize> = t.iter().enumerate().filter_map(|(i, token)|
        (token.quoted && token.value.eq_ignore_ascii_case("CompatToolMapping")
         && t.get(i+1).is_some_and(|v| !v.quoted && v.value == "{")).then_some(i+1)).collect();
    if blocks.len() > 1 { return Err("Ambiguous Steam mapping; rollback refused".into()); }
    let Some(&open) = blocks.first() else { return Ok(None) };
    let end = matching_close(&t, open)?;
    let mut found = None;
    let mut i = open + 1;
    while i < end {
        let value = t.get(i+1).ok_or("Malformed Steam mapping")?;
        let close = if !value.quoted && value.value == "{" { matching_close(&t, i+1)? } else { i+1 };
        if t[i].value == app_id {
            if found.is_some() { return Err("Duplicate game mapping; rollback refused".into()); }
            found = Some((t[i].start, t[close].end));
        }
        i = close + 1;
    }
    Ok(found)
}
fn rollback_text(undo: &ConfigUndo, current: &[u8]) -> Result<String, String> {
    let current = std::str::from_utf8(current).map_err(|e| e.to_string())?;
    let before = std::str::from_utf8(&undo.before).map_err(|e| e.to_string())?;
    let after = std::str::from_utf8(&undo.after).map_err(|e| e.to_string())?;
    let expected = mapping_range(after, &undo.app_id)?.ok_or("Expected game mapping missing")?;
    let live = mapping_range(current, &undo.app_id)?.ok_or("Game mapping changed; rollback refused")?;
    if current[live.0..live.1] != after[expected.0..expected.1] {
        return Err("This game's configuration changed since the fix. Rollback refused to preserve newer settings; the original backup remains available.".into());
    }
    let original = mapping_range(before, &undo.app_id)?.map(|(a,b)| &before[a..b]).unwrap_or("");
    let mut restored = current.to_string();
    restored.replace_range(live.0..live.1, original);
    scanner::parse_vdf(&restored)?;
    Ok(restored)
}

pub(crate) fn select_tracked(app_id:String,library_path:String,confirmed:bool, expected_tool: Option<&str>)->Result<(AutoSelection, Option<ConfigUndo>),String>{
    if !confirmed {return Err("Enable auto-selection or approve this game before changing its Proton version".into());}
    if app_id.is_empty() || !app_id.bytes().all(|b|b.is_ascii_digit()) {return Err("Invalid Steam app ID".into());}
    let _lock=CONFIG_LOCK.lock().map_err(|_|"Auto-selection lock unavailable")?;
    repairs::require_steam_closed()?;
    let scan=scanner::scan();
    let game=scan.games.iter().find(|g|g.app_id==app_id && g.library_path==library_path)
        .ok_or("Game not present in a freshly scanned Steam library")?;
    let diagnosis=diagnostics::diagnose(&scan);
    let advisor=advisor::recommend(&scan,&diagnosis);
    let rec=advisor.games.iter().find(|g|g.app_id==app_id && g.library_path==library_path)
        .ok_or("No fresh Proton recommendation for this game")?;
    if rec.status!="try-version" {return Err(format!("No safe automatic version change recommended: {}",rec.reason));}
    // Diagnostics must be recent: old logs could refer to a previous runtime.
    let detail=diagnosis.games.iter().find(|g|g.app_id==app_id && g.library_path==library_path)
        .ok_or("No game diagnostic record")?;
    let modified=detail.modified_unix_seconds.ok_or("No dated Proton log available; launch with logging first")?;
    let now=SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs();
    if modified>now+300 || now.saturating_sub(modified)>48*3600 {
        return Err("Proton log is old or has an invalid timestamp. Create a new log before auto-selecting".into());
    }
    let requested=rec.suggested_tool.as_deref().ok_or("No suitable Proton version installed")?;
    if expected_tool.is_some_and(|expected| expected != requested) {
        return Err("Recommendation changed before the write. Diagnose again.".into());
    }
    let available = tools_for_library(&scan, &library_path)?;
    let selected=available.iter().find(|t|t.name==requested)
        .ok_or("Recommended Proton is not installed for this game’s Steam installation")?;
    let id=internal_id(&selected.name,Path::new(&selected.path))?;
    let config=config_for_library(&scan,&library_path)?;
    let (backup_path,changed)=replace_config(&config,&app_id,&id,true)?;
    let undo = if changed {
        let before = fs::read(&backup_path).map_err(|e| format!("Selection saved, but cannot read rollback backup: {e}"))?;
        let original = std::str::from_utf8(&before).map_err(|e| e.to_string())?;
        let (after, _) = replacement_text(original, &app_id, &id)?;
        Some(ConfigUndo { path: config, app_id: app_id.clone(), before, after: after.into_bytes() })
    } else { None };
    Ok((AutoSelection{game_name:game.name.clone(),app_id,tool_name:selected.name.clone(),tool_id:id,
        message:if changed {"Proton version selected for this game. Restart Steam and test the game. The previous config was backed up.".into()}
            else {"This Proton version was already configured. No file changes were needed.".into()},
        backup_path,changed}, undo))
}

#[cfg(test)] mod tests {
    use super::*;
    const CONFIG:&str=r#""InstallConfigStore" { "Software" { "Valve" { "Steam" { "CompatToolMapping" {
        "0" { "name" "proton_experimental" } // user default
        "123" { "name" "proton_9" "config" "" "priority" "250" }
    } } } } }"#;
    #[test] fn manual_choices_follow_library_owner_and_resolve_links() {
        let dir = std::env::temp_dir().join(format!("pm-tool-choices-{}-{}", std::process::id(), unix_stamp().unwrap()));
        let native = dir.join("native"); let flatpak = dir.join("flatpak"); let external = dir.join("external");
        for root in [&native, &flatpak] {
            fs::create_dir_all(root.join("config")).unwrap();
            fs::create_dir_all(root.join("steamapps")).unwrap();
            fs::create_dir_all(root.join("compatibilitytools.d/GE")).unwrap();
            fs::write(root.join("config/config.vdf"), CONFIG).unwrap();
        }
        fs::create_dir_all(external.join("steamapps/common/Proton")).unwrap();
        fs::write(native.join("steamapps/libraryfolders.vdf"), format!(r#""libraryfolders" {{ "1" {{ "path" "{}" }} }}"#, external.display())).unwrap();
        let escaped = native.join("compatibilitytools.d/other-installation");
        std::os::unix::fs::symlink(flatpak.join("compatibilitytools.d/GE"), &escaped).unwrap();
        let paths = [native.join("compatibilitytools.d/GE"), flatpak.join("compatibilitytools.d/GE"), external.join("steamapps/common/Proton"), escaped];
        let scan = scanner::ScanReport {
            installations: vec![scanner::SteamInstallation { kind: "Native".into(), path: native.to_string_lossy().into() }, scanner::SteamInstallation { kind: "Flatpak".into(), path: flatpak.to_string_lossy().into() }],
            libraries: vec![], games: vec![], warnings: vec![],
            compatibility_tools: paths.iter().map(|p| scanner::CompatibilityTool { name: "Proton".into(), source: "Custom".into(), path: p.to_string_lossy().into() }).collect(),
        };
        let native_choices = tools_for_library(&scan, external.to_str().unwrap()).unwrap();
        assert_eq!(native_choices.iter().map(|t| t.path.as_str()).collect::<Vec<_>>(), vec![paths[0].to_str().unwrap(), paths[2].to_str().unwrap()]);
        let flatpak_choices = tools_for_library(&scan, flatpak.to_str().unwrap()).unwrap();
        assert!(flatpak_choices.iter().all(|t| fs::canonicalize(&t.path).unwrap() == paths[1]));
        assert!(!flatpak_choices.is_empty());
        fs::write(flatpak.join("steamapps/libraryfolders.vdf"), format!(r#""libraryfolders" {{ "1" {{ "path" "{}" }} }}"#, external.display())).unwrap();
        assert!(tools_for_library(&scan, external.to_str().unwrap()).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn automatic_advice_uses_only_the_owning_steam_installation() {
        let dir=std::env::temp_dir().join(format!("pm-auto-owner-{}-{}",std::process::id(),unix_stamp().unwrap()));
        let native=dir.join("native"); let flatpak=dir.join("flatpak");
        for root in [&native,&flatpak] {
            fs::create_dir_all(root.join("config")).unwrap();
            fs::create_dir_all(root.join("steamapps")).unwrap();
            fs::write(root.join("config/config.vdf"),CONFIG).unwrap();
        }
        let local=native.join("compatibilitytools.d/GE-Proton10-1");
        let foreign=flatpak.join("compatibilitytools.d/Proton Experimental");
        for path in [&local,&foreign] {fs::create_dir_all(path).unwrap();}
        let game=scanner::Game{app_id:"123".into(),name:"Fixture".into(),library_path:native.to_string_lossy().into(),install_path:String::new(),install_path_exists:true,manifest_path:String::new(),prefix_path:String::new(),prefix_exists:false,build_id:None,last_played_at:None,configured_tool:Some("proton_9".into())};
        let mut scan=scanner::ScanReport{installations:vec![scanner::SteamInstallation{kind:"Native".into(),path:native.to_string_lossy().into()},scanner::SteamInstallation{kind:"Flatpak".into(),path:flatpak.to_string_lossy().into()}],libraries:vec![],games:vec![game.clone()],warnings:vec![],compatibility_tools:vec![
            scanner::CompatibilityTool{name:"Proton Experimental".into(),path:foreign.to_string_lossy().into(),source:"Custom".into()},
            scanner::CompatibilityTool{name:"GE-Proton10-1".into(),path:local.to_string_lossy().into(),source:"Custom".into()}]};
        let detail=diagnostics::GameDiagnostics{app_id:"123".into(),library_path:game.library_path.clone(),log_path:None,log_size:None,modified_unix_seconds:None,truncated:false,status:"review".into(),notes:vec![],findings:vec![diagnostics::Finding{code:"wine-exception".into(),severity:"error".into(),title:"Exception".into(),description:String::new(),recommendation:String::new(),evidence:vec![]}]};
        let advice=advisor::advise_for_library(&scan,&game,Some(&detail));
        assert_eq!(advice.suggested_tool.as_deref(),Some("GE-Proton10-1"));
        assert!(!advice.alternatives.contains(&"Proton Experimental".into()));
        scan.compatibility_tools.retain(|t|t.path.as_str()!=local.to_str().unwrap());
        assert_eq!(advisor::advise_for_library(&scan,&game,Some(&detail)).status,"need-proton");
        fs::write(flatpak.join("steamapps/libraryfolders.vdf"),format!(r#""libraryfolders" {{ "1" {{ "path" "{}" }} }}"#,native.display())).unwrap();
        let ambiguous=advisor::advise_for_library(&scan,&game,Some(&detail));
        assert_eq!(ambiguous.status,"need-information");assert!(ambiguous.suggested_tool.is_none());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn configuration_writes_exclude_other_processes_and_reject_stale_snapshots() {
        use std::{io::{BufRead,BufReader},process::{Command,Stdio}};
        let dir=std::env::temp_dir().join(format!("pm-config-lock-{}-{}",std::process::id(),unix_stamp().unwrap()));
        fs::create_dir(&dir).unwrap();let path=dir.join("config.vdf");fs::write(&path,CONFIG).unwrap();
        let lease=crate::coordination::config(&path).unwrap();drop(lease);
        let mut child=Command::new("python3").args(["-c","import fcntl,sys; f=open(sys.argv[1],'r+'); fcntl.flock(f,fcntl.LOCK_EX); print('locked',flush=True); sys.stdin.read()"])
            .arg(dir.join(".config.vdf.proton-medic.lock")).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        let mut ready=String::new();BufReader::new(child.stdout.take().unwrap()).read_line(&mut ready).unwrap();assert_eq!(ready.trim(),"locked");
        assert!(replace_config(&path,"123","proton_10",false).is_err());assert_eq!(fs::read_to_string(&path).unwrap(),CONFIG);
        drop(child.stdin.take());assert!(child.wait().unwrap().success());
        replace_config(&path,"123","proton_10",false).unwrap();let newer=fs::read(&path).unwrap();
        let stale=replacement_text(CONFIG,"456","proton_9").unwrap().0;
        assert!(write_config(&path,CONFIG.as_bytes(),stale.as_bytes(),false).unwrap_err().contains("changed"));assert_eq!(fs::read(&path).unwrap(),newer);
        replace_config(&path,"456","proton_9",false).unwrap();let final_text=fs::read_to_string(&path).unwrap();
        assert!(final_text.contains("proton_10"));assert!(final_text.contains("\"456\""));
        let lease=crate::coordination::config(&path).unwrap();assert!(replace_config(&path,"123","proton_9",false).is_err());drop(lease);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn edits_only_game_name_value_and_preserves_comments() {
        let (next,changed)=replacement_text(CONFIG,"123","proton_10").unwrap();
        assert!(changed);assert!(next.contains("\"123\" { \"name\" \"proton_10\""));
        assert!(next.contains("// user default"));assert!(next.contains("\"proton_experimental\""));
    }
    #[test] fn inserts_missing_game_without_changing_default() {
        let (next,changed)=replacement_text(CONFIG,"456","proton_experimental").unwrap();
        assert!(changed);assert!(next.contains("\"456\" { \"name\" \"proton_experimental\""));
        assert!(next.contains("\"123\" { \"name\" \"proton_9\""));
    }
    #[test] fn no_change_if_same_tool(){assert!(!replacement_text(CONFIG,"123","proton_9").unwrap().1);}
    #[test] fn rejects_untrusted_tool_id(){assert!(replacement_text(CONFIG,"123","proton_9\" } \"0\"").is_err());}
    #[test] fn creates_compat_mapping_in_fresh_steam_config(){
        let bare=r#""InstallConfigStore" { "Software" { "Valve" { "Steam" { "Other" "value" } } } }"#;
        let (modified,changed)=replacement_text(bare,"123","proton_10").unwrap();
        assert!(changed);assert!(modified.contains("\"CompatToolMapping\""));
        assert!(modified.contains("\"proton_10\""));
        assert!(modified.contains("\"Other\" \"value\""));
    }
    #[test] fn rejects_unrecognized_steam_layout(){assert!(replacement_text("\"Steam\" {}","123","proton_10").is_err());}
    #[test] fn manifest_id_is_used_for_custom_proton(){
        let dir=std::env::temp_dir().join(format!("pm-proton-{}-{}",std::process::id(),unix_stamp().unwrap()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("compatibilitytool.vdf"),r#""compat_tools" { "GE-Proton11-4-x86_64" { "install_path" "." } }"#).unwrap();
        assert_eq!(internal_id("SomethingElse",&dir).unwrap(),"GE-Proton11-4-x86_64");
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test] fn backup_and_write_is_atomic_for_fixture(){
        let dir=std::env::temp_dir().join(format!("pm-config-{}-{}",std::process::id(),unix_stamp().unwrap()));
        fs::create_dir_all(&dir).unwrap();let file=dir.join("config.vdf");fs::write(&file,CONFIG).unwrap();
        let (bak,changed)=replace_config(&file,"123","proton_10",false).unwrap();
        assert!(changed);assert_eq!(fs::read_to_string(&bak).unwrap(),CONFIG);
        assert!(fs::read_to_string(&file).unwrap().contains("\"proton_10\""));
        fs::remove_dir_all(&dir).unwrap();
    }
    #[test] fn tracked_rollback_preserves_both_configurations() {
        let dir=std::env::temp_dir().join(format!("pm-undo-{}-{}",std::process::id(),unix_stamp().unwrap()));
        fs::create_dir_all(&dir).unwrap();
        let path=dir.join("config.vdf"); fs::write(&path,CONFIG).unwrap();
        let (backup, _) = replace_config(&path,"123","proton_10",false).unwrap();
        let after = fs::read(&path).unwrap();
        let undo = ConfigUndo { path: path.clone(), app_id: "123".into(), before: CONFIG.as_bytes().to_vec(), after: after.clone() };
        let displaced = rollback_file(&undo, false).unwrap();
        assert_eq!(fs::read(&path).unwrap(), CONFIG.as_bytes());
        assert_eq!(fs::read(backup).unwrap(), CONFIG.as_bytes());
        assert_eq!(fs::read(displaced).unwrap(), after);
        assert!(rollback_file(&undo, false).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test] fn rollback_refuses_newer_changes_and_symlinks() {
        let dir=std::env::temp_dir().join(format!("pm-undo-conflict-{}-{}",std::process::id(),unix_stamp().unwrap()));
        fs::create_dir_all(&dir).unwrap();
        let path=dir.join("config.vdf");
        let undo=ConfigUndo { path: path.clone(), app_id: "123".into(), before: CONFIG.as_bytes().to_vec(), after: b"expected".to_vec() };
        fs::write(&path,"new user settings").unwrap();
        assert!(rollback_file(&undo,false).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(),"new user settings");
        fs::remove_file(&path).unwrap();
        let real=dir.join("real");fs::write(&real,"expected").unwrap();
        std::os::unix::fs::symlink(&real,&path).unwrap();
        assert!(rollback_file(&undo,false).is_err());
        assert_eq!(fs::read_to_string(real).unwrap(),"expected");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test] fn rollback_preserves_unrelated_steam_edits() {
        let (after, _) = replacement_text(CONFIG, "123", "proton_10").unwrap();
        let undo=ConfigUndo { path: PathBuf::new(), app_id: "123".into(), before: CONFIG.as_bytes().to_vec(), after: after.as_bytes().to_vec() };
        let (current, _) = replacement_text(&after, "456", "proton_11").unwrap();
        let restored=rollback_text(&undo,current.as_bytes()).unwrap();
        assert!(restored.contains("\"123\" { \"name\" \"proton_9\""));
        assert!(restored.contains("\"456\" { \"name\" \"proton_11\""));
        let (conflict, _) = replacement_text(&current,"123","proton_11").unwrap();
        assert!(rollback_text(&undo,conflict.as_bytes()).is_err());
    }
    #[test] fn rollback_removes_only_newly_added_override() {
        let (after, _) = replacement_text(CONFIG,"456","proton_10").unwrap();
        let undo=ConfigUndo { path: PathBuf::new(), app_id: "456".into(), before: CONFIG.as_bytes().to_vec(), after: after.as_bytes().to_vec() };
        let restored=rollback_text(&undo,after.as_bytes()).unwrap();
        assert!(mapping_range(&restored,"456").unwrap().is_none());
        assert!(mapping_range(&restored,"123").unwrap().is_some());
    }

}
