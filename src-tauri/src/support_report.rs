//! Reviewed, local-only diagnostic snapshots. No raw logs or launch arguments.
use crate::{diagnostics, inventory, windows_games};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, path::Path, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use tauri_plugin_dialog::{DialogExt, FilePath};

const MAX_BYTES: usize = 2 * 1024 * 1024;
const STEAM_LIMIT: usize = 100;
const WINDOWS_LIMIT: usize = 50;
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options { pub steam: bool, pub windows: bool, pub history: bool, pub redact_paths: bool }
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview { pub token: u64, pub content: String }
#[derive(Default)]
struct Snapshots { next: u64, current: Option<Preview> }
impl Snapshots {
    fn remember(&mut self, content: String) -> Result<Preview, String> {
        if content.len() > MAX_BYTES { return Err("Report exceeds 2 MiB. Exclude launch history and try again.".into()); }
        self.next = self.next.checked_add(1).ok_or("Report counter exhausted")?;
        let preview = Preview { token: self.next, content };
        self.current = Some(preview.clone()); Ok(preview)
    }
    fn reviewed(&self, token: u64) -> Result<String, String> {
        self.current.as_ref().filter(|p|p.token == token).map(|p|p.content.clone()).ok_or("This preview is no longer available. Generate it again.".into())
    }
}
static SNAPSHOTS: Mutex<Snapshots> = Mutex::new(Snapshots { next: 0, current: None });
// Omit the entire path-bearing field rather than guessing where a path with spaces ends.
// Projection below also excludes raw logs, evidence, messages, environment and arguments.
fn redact(value: &mut Value) {
    match value {
        Value::String(s) if s.contains('/') || s.contains('\\') || s.starts_with('~') => *s = "[redacted path]".into(),
        Value::Array(values) => values.iter_mut().for_each(redact),
        Value::Object(values) => values.values_mut().for_each(redact),
        _ => {}
    }
}
fn steam_game(game: &crate::scanner::Game, diagnostic: Option<&diagnostics::GameDiagnostics>) -> Value {
    json!({"appId":game.app_id,"name":game.name,"libraryPath":game.library_path,"installPath":game.install_path,
        "installPathExists":game.install_path_exists,"prefixPath":game.prefix_path,"prefixExists":game.prefix_exists,
        "configuredTool":game.configured_tool,"diagnostics":diagnostic.map(|d|json!({"status":d.status,"logPath":d.log_path,
            "logSize":d.log_size,"truncated":d.truncated,"modifiedUnixSeconds":d.modified_unix_seconds,
            "findings":d.findings.iter().map(|f|json!({"code":f.code,"severity":f.severity,"title":f.title,"recommendation":f.recommendation})).collect::<Vec<_>>() }))})
}
fn windows_game(game: &windows_games::Game, runtime: Option<&str>) -> Value {
    json!({"name":game.name,"runtime":runtime,"state":game.state,"exitCode":game.exit_code,
        "executable":game.game_exe,"prefixDirectory":game.prefix_directory,"workingDirectory":game.working_directory,
        "lastLaunchedAt":game.last_launched_at})
}
fn launch_history(history: windows_games::history::History) -> Value {
    json!({"unreadableRecordWarnings":history.warnings.len(),"entries":history.entries.into_iter().map(|e|json!({
        "kind":e.kind,"runtime":e.runtime,"startedAt":e.started_at,"endedAt":e.ended_at,"state":e.state,
        "exitCode":e.exit_code,"summaries":e.summaries,"partialLogs":e.partial_logs})).collect::<Vec<_>>()})
}
pub fn prepare(options: Options) -> Result<Preview, String> {
    let mut scan = (*inventory::steam()).clone();
    let total_steam = scan.games.len();
    scan.games.truncate(if options.steam { STEAM_LIMIT } else { 0 });
    let diagnostics = diagnostics::diagnose(&scan);
    let mut report = json!({"schemaVersion":1,"appVersion":env!("CARGO_PKG_VERSION"),
        "generatedAtUnixSeconds":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_|"Invalid system time")?.as_secs(),
        "pathsRedacted":options.redact_paths,
        "scope":{"steam":options.steam,"windows":options.windows,"windowsHistory":options.windows && options.history},
        "excluded":["raw logs","log evidence","launch arguments","environment variables","account data","removed game profiles","resource samples"],
        "notes":["Game names, runtime names and selected launch timestamps are included. Review before sharing.",
            "Path-bearing text fields are replaced in full when redaction is enabled.",
            "Runtime exits do not confirm gameplay. No active Vulkan probe is performed.",
            "Steam inventory may be cached for up to 30 seconds. Log findings are collected for this report."],
        "system":{"os":diagnostics.system.os,"kernel":diagnostics.system.kernel,"architecture":diagnostics.system.arch,
            "graphicsDevices":diagnostics.system.gpu_devices,"vulkanUtilityFound":diagnostics.system.vulkaninfo_path.is_some(),
            "vulkanDriverManifestCount":diagnostics.system.icd_manifests.len()},
        "steam":null,"windows":null});
    if options.steam {
        report["steam"] = json!({"totalGames":total_steam,"omittedGames":total_steam.saturating_sub(scan.games.len()),
            "scanWarningCount":scan.warnings.len(),"diagnosticWarningCount":diagnostics.warnings.len(),
            "games":scan.games.iter().map(|g|steam_game(g,diagnostics.games.iter().find(|d|d.app_id == g.app_id && d.library_path == g.library_path))).collect::<Vec<_>>()});
    }
    if options.windows {
        report["windows"] = match windows_games::list() {
            Err(_) => json!({"status":"unavailable","note":"Windows library could not be read. Retry from Windows Games."}),
            Ok(library) => {
                let games = library.games.iter().take(WINDOWS_LIMIT).map(|g| {
                    let mut value = windows_game(g, library.runtimes.iter().find(|r|r.id == g.runtime_id).map(|r|r.name.as_str()));
                    if options.history {
                        value["history"] = match windows_games::history::list(g.id.clone()) {
                            Ok(history) => launch_history(history),
                            Err(_) => json!({"status":"unavailable"})
                        };
                    }
                    value
                }).collect::<Vec<_>>();
                json!({"status":"available","totalGames":library.games.len(),"omittedGames":library.games.len().saturating_sub(games.len()),
                    "warningCount":library.warnings.len(),"historyLimitPerGame":20,"games":games})
            }
        };
    }
    if options.redact_paths { redact(&mut report); }
    let mut content = serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?; content.push('\n');
    SNAPSHOTS.lock().map_err(|_|"Report preview unavailable")?.remember(content)
}
fn write_new(path: &Path, content: &str) -> Result<(), String> {
    if path.extension().and_then(|s|s.to_str()) != Some("json") { return Err("Choose a filename ending in .json.".into()); }
    if content.len() > MAX_BYTES { return Err("Report exceeds the size limit.".into()); }
    // create_new refuses existing files and symlinks; never truncate a user's file.
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
        .map_err(|e|format!("Could not create report. Choose a new filename: {e}"))?;
    file.write_all(content.as_bytes()).and_then(|_|file.sync_all())
        .map_err(|e|format!("Report could not be fully saved; a partial file may remain. Choose a new filename to retry: {e}"))
}
pub fn save(app: tauri::AppHandle, token: u64, confirmed: bool) -> Result<Option<String>, String> {
    if !confirmed { return Err("Review and approve the report before saving.".into()); }
    let content = SNAPSHOTS.lock().map_err(|_|"Report preview unavailable")?.reviewed(token)?;
    let _activity = crate::app_update::activity()?;
    let path = app.dialog().file().set_title("Save troubleshooting report — choose a new file")
        .set_file_name(format!("proton-medic-report-{}.json",SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_|"Invalid system time")?.as_secs()))
        .add_filter("JSON report", &["json"]).blocking_save_file();
    let Some(path) = path else { return Ok(None) };
    let FilePath::Path(path) = path else { return Err("Choose a local file location.".into()) };
    write_new(&path, &content)?;
    Ok(Some(path.to_string_lossy().into_owned()))
}
#[cfg(test)] mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test] fn redaction_removes_complete_paths_in_nested_fields() {
        let mut value=json!({"name":"Game","paths":["/home/Bless/My Games/game.exe","Z:\\home\\Bless\\game.exe","~/secret","prefix /mnt/drive with spaces/file and more","file:///home/Bless/a"],"exit":7,"empty":null});
        redact(&mut value);let text=value.to_string();assert!(!text.contains("Bless"));assert!(!text.contains("/mnt"));assert!(!text.contains("secret"));assert_eq!(value["name"],"Game");assert_eq!(value["exit"],7);
    }
    #[test] fn windows_projection_excludes_arguments_messages_and_internal_ids() {
        let game=windows_games::Game{id:"private-id".into(),name:"Game".into(),runtime_id:"/home/Bless/runtime".into(),game_exe:Some("/home/Bless/game.exe".into()),state:"game-exited".into(),exit_code:Some(7),message:"token=secret".into(),directory:"private".into(),windows_drive:"private".into(),prefix_directory:"/mnt/secret".into(),log_directory:Some("private".into()),launch_arguments:vec!["password=secret".into()],working_directory:None,last_launched_at:None};
        let mut value=windows_game(&game,Some("Wine"));redact(&mut value);let text=value.to_string();for secret in ["Bless","secret","private","password","token"]{assert!(!text.contains(secret));}assert_eq!(value["exitCode"],7);
    }
    #[test] fn steam_projection_keeps_findings_without_log_evidence_or_notes() {
        let game=crate::scanner::Game{app_id:"123".into(),name:"Game".into(),library_path:"/mnt/private".into(),install_path:"/home/Bless/game".into(),install_path_exists:true,manifest_path:"/private/manifest".into(),prefix_path:"/home/Bless/pfx".into(),prefix_exists:true,build_id:None,configured_tool:Some("Proton".into()),last_played_at:None};
        let diagnostic=diagnostics::GameDiagnostics{app_id:"123".into(),library_path:game.library_path.clone(),log_path:Some("/home/Bless/log".into()),log_size:Some(123),modified_unix_seconds:None,truncated:true,status:"review".into(),findings:vec![diagnostics::Finding{code:"memory".into(),severity:"warning".into(),title:"Memory allocation failure".into(),description:"unused".into(),recommendation:"Check available memory".into(),evidence:vec!["password=secret".into()]}],notes:vec!["token=private".into()]};
        let mut value=steam_game(&game,Some(&diagnostic));redact(&mut value);let text=value.to_string();for secret in ["Bless","private","secret","password","token"]{assert!(!text.contains(secret));}assert_eq!(value["diagnostics"]["findings"][0]["code"],"memory");assert_eq!(value["diagnostics"]["truncated"],true);
    }
    #[test] fn history_projection_excludes_boot_identity_and_freeform_errors() {
        let entry:windows_games::history::Entry=serde_json::from_value(json!({"id":"private-id","kind":"game","runtime":"Wine","startedAt":123,"endedAt":456,"state":"process-failed","exitCode":7,"message":"password=secret","summaries":["Memory allocation failure"],"partialLogs":true,"bootId":"private-boot"})).unwrap();
        let value=launch_history(windows_games::history::History{entries:vec![entry],warnings:vec!["/home/Bless/private".into()]});let text=value.to_string();assert!(!text.contains("private"));assert!(!text.contains("secret"));assert_eq!(value["unreadableRecordWarnings"],1);assert_eq!(value["entries"][0]["exitCode"],7);
    }
    #[test] fn snapshots_preserve_exact_reviewed_bytes_and_reject_stale_tokens() {
        let mut snapshots=Snapshots::default();let first=snapshots.remember("first\n".into()).unwrap();assert_eq!(snapshots.reviewed(first.token).unwrap(),"first\n");
        let second=snapshots.remember("second\n".into()).unwrap();assert!(snapshots.reviewed(first.token).is_err());assert_eq!(snapshots.reviewed(second.token).unwrap(),"second\n");assert!(snapshots.remember("x".repeat(MAX_BYTES+1)).is_err());
    }
    #[test] fn saved_report_is_private_and_never_overwrites_existing_files_or_symlinks() {
        let dir=std::env::temp_dir().join(format!("pm-report-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir(&dir).unwrap();
        let path=dir.join("report.json");write_new(&path,"{\"reviewed\":true}\n").unwrap();assert_eq!(std::fs::read_to_string(&path).unwrap(),"{\"reviewed\":true}\n");assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,0o600);
        assert!(write_new(&path,"different").is_err());assert_eq!(std::fs::read_to_string(&path).unwrap(),"{\"reviewed\":true}\n");
        let link=dir.join("link.json");symlink(&path,&link).unwrap();assert!(write_new(&link,"different").is_err());assert!(write_new(&dir.join("report.txt"),"x").is_err());std::fs::remove_dir_all(dir).unwrap();
    }
}
