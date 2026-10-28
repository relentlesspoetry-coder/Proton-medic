//! Phase 4: bounded, user-triggered Steam launch observation.
//!
//! We never declare a game playable from a Steam CLI exit code or from
//! the absence of known Proton log errors. The user confirms playability.
//! No arbitrary command/path is accepted from the frontend.
use crate::{diagnostics, scanner};
use serde::Serialize;
use std::{env, fs, path::{Path, PathBuf}, process::Stdio, sync::Mutex,
    thread, time::{Duration, Instant, SystemTime}};

static LAUNCH_LOCK: Mutex<()> = Mutex::new(());
const MIN_SECONDS: u64 = 5;
const MAX_SECONDS: u64 = 45;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub app_id: String,
    pub library_path: String,
    pub game_name: String,
    pub status: String,
    pub detail: String,
    pub observed_seconds: u64,
    pub launcher_exit_code: Option<i32>,
    pub fresh_log: bool,
    pub finding_codes: Vec<String>,
    pub manually_confirmed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LogFingerprint {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

fn log_locations(home: &Path, app_id: &str) -> Vec<PathBuf> {
    let filename = format!("steam-{app_id}.log");
    [home.join(&filename),
        home.join(".var/app/com.valvesoftware.Steam").join(&filename),
        home.join(".var/app/com.valvesoftware.Steam/data").join(filename)].into()
}

fn fingerprint(path: &Path) -> Option<LogFingerprint> {
    // Never follow symlinks while inspecting a log.
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.file_type().is_symlink() { return None; }
    Some(LogFingerprint { path: path.to_path_buf(), modified: meta.modified().ok()?, len: meta.len() })
}

fn fresh_log(home: &Path, app_id: &str, before: &[LogFingerprint], start: SystemTime)
    -> Option<LogFingerprint> {
    // Only a log that actually changed during THIS test counts as new evidence.
    // A different Steam installation's stale/future-dated log must not mask it.
    log_locations(home, app_id).into_iter().filter_map(|p| fingerprint(&p))
        .filter(|log| changed_log(before, log, start))
        .max_by_key(|log| log.modified)
}

fn changed_log(before: &[LogFingerprint], after: &LogFingerprint, start: SystemTime) -> bool {
    // A stale log must never count as evidence from this test. File timestamp
    // granularity varies by filesystem; a changed length also counts.
    let changed = before.iter().find(|old| old.path == after.path)
        .is_none_or(|old| old.modified != after.modified || old.len != after.len);
    changed && after.modified.duration_since(start).is_ok()
}

fn assess(fresh: bool, severity: &[String], launch_exit: Option<i32>, unreadable: bool) -> (&'static str, &'static str) {
    if fresh && severity.iter().any(|s| s == "error") {
        ("log-error", "A new Proton log contains error messages. Review the findings before retrying.")
    } else if fresh && severity.iter().any(|s| s == "warning") {
        ("log-warning", "A new log contains warnings; the game may or may not be playable.")
    } else if launch_exit.is_some_and(|code| code != 0) {
        ("launcher-error", "Steam returned an error while handling the launch request.")
    } else if fresh && unreadable {
        ("log-unreadable", "A new Proton log was found but could not be analyzed. Check the log path, permissions, or size.")
    } else if fresh {
        ("log-no-known-errors", "A new log appeared without recognized errors. Please confirm whether the game is playable.")
    } else {
        ("no-fresh-log", "A launch request was sent to Steam, but no new Proton log was captured. This is not a pass or fail.")
    }
}

fn launch_and_observe(binary: &Path, args: &[String], seconds: u64,
    before: Vec<LogFingerprint>, home: &Path, app_id: &str,
    scan: &scanner::ScanReport, library_path: &str,
) -> Result<LaunchResult, String> {
    let game = scan.games.iter().find(|g| g.app_id == app_id && g.library_path == library_path)
        .ok_or("Game was not found in the current Steam scan")?;
    // Explicit argument vector; no shell, script, or user-supplied executable.
    let start_wall = SystemTime::now();
    let mut child = crate::font_runtime::child_command(binary).args(args)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Steam launcher could not start: {e}"))?;
    let start = Instant::now();
    let mut exit_code = None;
    while start.elapsed() < Duration::from_secs(seconds) {
        if exit_code.is_none() {
            if let Some(status) = child.try_wait().map_err(|e| format!("Could not inspect Steam launcher: {e}"))? {
                exit_code = Some(status.code().unwrap_or(-1));
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    // IMPORTANT: Never kill the child on timeout. Steam can be a long-running
    // client, not a short-lived helper. Dropping Child detaches safely.
    let fresh_log = fresh_log(home, app_id, &before, start_wall
        .checked_sub(Duration::from_secs(1)).unwrap_or(SystemTime::UNIX_EPOCH));
    let mut codes = Vec::new();
    let mut severity = Vec::new();
    let mut unreadable = false;
    if let Some(fresh) = fresh_log.as_ref() {
        // Parse the exact fresh file, rather than whichever file happens to
        // have the highest modification timestamp across multiple Steam roots.
        let game_diags = diagnostics::analyze_exact_log(&fresh.path, app_id, library_path);
        unreadable = game_diags.status == "unreadable";
        for finding in &game_diags.findings {
            codes.push(finding.code.clone());
            severity.push(finding.severity.clone());
        }
    }
    let (status, detail) = assess(fresh_log.is_some(), &severity, exit_code, unreadable);
    Ok(LaunchResult {
        app_id: app_id.into(), library_path: library_path.into(), game_name: game.name.clone(),
        status: status.into(), detail: detail.into(), observed_seconds: seconds,
        launcher_exit_code: exit_code, fresh_log: fresh_log.is_some(), finding_codes: codes,
        manually_confirmed: false,
    })
}

pub fn test_game(app_id: String, library_path: String, seconds: u64, confirmed: bool) -> Result<LaunchResult, String> {
    if !confirmed { return Err("You must approve launching the selected game".into()); }
    if !(MIN_SECONDS..=MAX_SECONDS).contains(&seconds) { return Err("Observation duration must be 5–45 seconds".into()); }
    if app_id.is_empty() || !app_id.bytes().all(|b| b.is_ascii_digit()) { return Err("Invalid Steam App ID".into()); }
    let _lock = LAUNCH_LOCK.try_lock().map_err(|_| "Another launch test is already running")?;
    let scan = scanner::scan();
    let game = scan.games.iter().find(|g| g.app_id == app_id && g.library_path == library_path)
        .ok_or("Selected game was not found in a current Steam scan")?;
    if !game.install_path_exists { return Err("Game installation is missing; cannot launch it".into()); }
    let library = scan.libraries.iter().find(|l| l.path == library_path)
        .ok_or("Steam library is unavailable")?;
    let home = env::var_os("HOME").map(PathBuf::from).ok_or("HOME was not found")?;
    let before = log_locations(&home, &app_id).into_iter().filter_map(|p| fingerprint(&p)).collect();
    // Attempt log setup, but never make safe account selection a launch prerequisite.
    let logging = logging_for_launch(crate::repairs::require_steam_closed().is_err(),
        || crate::game_logging::prepare(app_id.clone(), library_path.clone(), true));
    // Existing logs and ambiguous account settings are retained.
    let flatpak = library.install_kind.eq_ignore_ascii_case("flatpak");
    let (launcher, args): (&str, Vec<String>) = if flatpak {
        ("flatpak", vec!["run".into(), "com.valvesoftware.Steam".into(), "-applaunch".into(), app_id.clone()])
    } else {
        ("steam", vec!["-applaunch".into(), app_id.clone()])
    };
    with_logging_status(logging, || launch_and_observe(Path::new(launcher), &args, seconds, before, &home, &app_id, &scan, &library_path))
}

// A running Steam client owns its current account and launch options. Never
// resolve or edit an account as a prerequisite for sending it a launch request.
fn logging_for_launch(steam_may_be_running: bool,
    setup: impl FnOnce() -> Result<crate::game_logging::Status, String>)
    -> Result<crate::game_logging::Status, String> {
    if steam_may_be_running {
        Ok(crate::game_logging::Status {
            state: "existing-settings".into(),
            message: "Launch options were left unchanged because Steam is running or could not be verified closed. Existing logs will still be checked.".into(),
            backup_path: None, can_restore: false,
        })
    } else { setup() }
}

// The logging warning is evidence about setup, not a launcher failure or a pass.
fn with_logging_status(logging: Result<crate::game_logging::Status, String>,
    launch: impl FnOnce() -> Result<LaunchResult, String>) -> Result<LaunchResult, String> {
    let warning = match logging {
        Ok(status) if status.state == "ready" => None,
        Ok(status) => Some(status.message),
        Err(error) => Some(error),
    };
    let mut result = launch()?;
    if let Some(warning) = warning {
        result.detail = format!("Automatic log setup was not completed: {warning} Launch continued with existing Steam settings. {}", result.detail);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::{Game, SteamLibrary, ScanReport};
    use std::{os::unix::fs::PermissionsExt, time::{UNIX_EPOCH, SystemTime}};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = env::temp_dir().join(format!("proton-medic-launch-fixture-{}-{}", std::process::id(),
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
            fs::create_dir(&path).unwrap(); Self(path)
        }
        fn launcher(&self, command: &str) -> PathBuf {
            let path = self.0.join("fake-steam");
            fs::write(&path, format!("#!/bin/sh\n{command}\n")).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap(); path
        }
        fn scan(&self) -> ScanReport {
            let library_path = self.0.to_string_lossy().to_string();
            ScanReport {installations: vec![],libraries: vec![SteamLibrary {
                path: library_path.clone(), steam_apps_path: self.0.join("steamapps").to_string_lossy().to_string(),
                install_kind: "Native".into() }],
                games: vec![Game {app_id:"98765".into(), name:"Disposable Fixture".into(),
                    library_path:library_path.clone(),install_path:self.0.to_string_lossy().to_string(),
                    install_path_exists:true,manifest_path:String::new(),prefix_path:String::new(),
                    prefix_exists:false,build_id:None,configured_tool:None}],
                compatibility_tools:vec![],warnings:vec![] }
        }
    }
    impl Drop for Fixture {fn drop(&mut self) {let _=fs::remove_dir_all(&self.0);}}
    #[test] fn running_steam_bypasses_account_lookup_and_log_configuration() {
        let status=logging_for_launch(true,||panic!("Must not resolve or edit a running Steam account")).unwrap();
        assert_eq!(status.state,"existing-settings");assert!(status.backup_path.is_none());
        assert_eq!(logging_for_launch(false,||Err("Setup was attempted".into())).unwrap_err(),"Setup was attempted");
    }
    #[test] fn account_ambiguity_still_executes_launcher_and_reports_warning() {
        let f=Fixture::new();let marker=f.0.join("launched");
        let launcher=f.launcher(&format!("touch '{}'",marker.display()));let scan=f.scan();
        let result=with_logging_status(Err("Cannot identify one most-recent Steam account".into()),
            || launch_and_observe(&launcher,&["-applaunch".into(),"98765".into()],1,
                vec![],&f.0,"98765",&scan,&scan.libraries[0].path)).unwrap();
        assert!(marker.exists());assert!(result.detail.contains("most-recent Steam account"));
        assert_eq!(result.status,"no-fresh-log");assert!(!result.manually_confirmed);
    }
    #[test] fn queued_logs_do_not_block_launch_and_launcher_errors_still_propagate() {
        let status=||Ok(crate::game_logging::Status{state:"waiting-for-steam".into(),message:"Close Steam for log setup".into(),backup_path:None,can_restore:false});
        let f=Fixture::new();let launcher=f.launcher("exit 7");let scan=f.scan();
        let result=with_logging_status(status(),|| launch_and_observe(&launcher,&[],1,
            vec![],&f.0,"98765",&scan,&scan.libraries[0].path)).unwrap();
        assert_eq!(result.status,"launcher-error");assert!(result.detail.contains("Close Steam"));
        assert_eq!(with_logging_status(status(),||Err("Executable missing".into())).unwrap_err(),"Executable missing");
    }
    #[test] fn reports_errors_without_claiming_a_game_works() {
        assert_eq!(assess(true,&["error".into()],Some(0),false).0,"log-error");
        assert_eq!(assess(true,&[],Some(0),false).0,"log-no-known-errors");
        assert_eq!(assess(true,&[],Some(0),true).0,"log-unreadable");
        assert_eq!(assess(false,&[],Some(0),false).0,"no-fresh-log");
        assert_eq!(assess(false,&[],Some(1),false).0,"launcher-error");
    }
    #[test] fn stale_log_does_not_count_as_a_new_test() {
        let f=Fixture::new();let path=f.0.join("steam-98765.log");fs::write(&path,"previous result").unwrap();
        let old=fingerprint(&path).unwrap();
        assert!(!changed_log(&[old.clone()], &old, SystemTime::now()));
    }
    #[test] fn ignores_newer_stale_flatpak_log_when_native_log_is_fresh() {
        let f = Fixture::new();
        let stale = f.0.join(".var/app/com.valvesoftware.Steam/steam-98765.log");
        fs::create_dir_all(stale.parent().unwrap()).unwrap();
        fs::write(&stale, "vk_error_incompatible_driver\n").unwrap();
        let future = SystemTime::now() + Duration::from_secs(120);
        fs::OpenOptions::new().write(true).open(&stale).unwrap()
            .set_times(fs::FileTimes::new().set_modified(future)).unwrap();
        let before = log_locations(&f.0, "98765").into_iter()
            .filter_map(|path| fingerprint(&path)).collect();
        let current = f.0.join("steam-98765.log");
        let launcher = f.launcher(&format!("printf 'vk_error_device_lost\\n' > '{}'", current.display()));
        let scan = f.scan();
        let result = launch_and_observe(&launcher, &["-applaunch".into(), "98765".into()], 1,
            before, &f.0, "98765", &scan, &scan.libraries[0].path).unwrap();
        assert!(result.fresh_log);
        assert_eq!(result.status, "log-error");
        assert!(result.finding_codes.contains(&"vulkan-device-lost".to_string()));
        assert!(!result.finding_codes.contains(&"vulkan-init".to_string()));
    }
    #[test] fn symlinked_log_is_not_accepted_as_new_evidence() {
        let f = Fixture::new();
        let destination = f.0.join("private-data");
        fs::write(&destination, "vk_error_device_lost\n").unwrap();
        std::os::unix::fs::symlink(&destination, f.0.join("steam-98765.log")).unwrap();
        let launcher = f.launcher("exit 0");
        let scan = f.scan();
        let result = launch_and_observe(&launcher, &["-applaunch".into(), "98765".into()], 1,
            vec![], &f.0, "98765", &scan, &scan.libraries[0].path).unwrap();
        assert!(!result.fresh_log);
        assert!(result.finding_codes.is_empty());
    }
    #[test] fn fake_steam_launcher_is_executed_without_real_game_files() {
        let f=Fixture::new(); let launcher=f.launcher("exit 0");let scan=f.scan();
        let result=launch_and_observe(&launcher,&["-applaunch".into(),"98765".into()],0,
            vec![],&f.0,"98765",&scan,&scan.libraries[0].path).unwrap();
        assert_eq!(result.status,"no-fresh-log");
        assert!(!result.manually_confirmed);
    }
    #[test] fn fake_steam_can_emit_new_log_but_not_prove_playability() {
        let f=Fixture::new();
        let log=f.0.join("steam-98765.log");
        let launcher=f.launcher(&format!("printf 'vk_error_device_lost\\n' > '{}'",log.display()));
        let scan=f.scan();
        let result=launch_and_observe(&launcher,&["-applaunch".into(),"98765".into()],1,
            vec![],&f.0,"98765",&scan,&scan.libraries[0].path).unwrap();
        assert!(result.fresh_log);
        assert!(!result.manually_confirmed);
        assert_eq!(result.status, "log-error");
        assert!(result.finding_codes.contains(&"vulkan-device-lost".to_string()));
    }
    #[test] fn new_vulkan_device_loss_is_reported_from_disposable_fixture() {
        let f = Fixture::new();
        let log = f.0.join("steam-98765.log");
        let launcher = f.launcher(&format!("printf 'vk_error_device_lost\\n' > '{}'", log.display()));
        let scan = f.scan();
        let result = launch_and_observe(&launcher, &["-applaunch".into(), "98765".into()], 1,
            vec![], &f.0, "98765", &scan, &scan.libraries[0].path).unwrap();
        assert!(result.fresh_log);
        assert_eq!(result.status, "log-error");
        assert!(result.finding_codes.contains(&"vulkan-device-lost".to_string()));
        assert!(!result.manually_confirmed);
    }
    #[test] fn fresh_log_does_not_count_as_automatic_playability_confirmation() {
        let f = Fixture::new();
        let log = f.0.join("steam-98765.log");
        let launcher = f.launcher(&format!("printf 'startup complete\\n' > '{}'", log.display()));
        let scan = f.scan();
        let result = launch_and_observe(&launcher, &["-applaunch".into(), "98765".into()], 1,
            vec![], &f.0, "98765", &scan, &scan.libraries[0].path).unwrap();
        assert_eq!(result.status, "log-no-known-errors");
        assert!(!result.manually_confirmed);
    }
    #[test] fn launcher_failure_is_reported_not_marked_playable() {
        let f=Fixture::new(); let launcher=f.launcher("exit 7");let scan=f.scan();
        let result=launch_and_observe(&launcher,&["-applaunch".into(),"98765".into()],1,
            vec![],&f.0,"98765",&scan,&scan.libraries[0].path).unwrap();
        assert_eq!(result.status,"launcher-error");
        assert_eq!(result.launcher_exit_code,Some(7));
        assert!(!result.manually_confirmed);
    }
    #[test] fn launch_requires_approval_before_scanning_or_spawning() {
        let error=test_game("98765".into(),"/no/such/library".into(),10,false).unwrap_err();
        assert!(error.contains("approve"));
        let error=test_game("98765".into(),"/no/such/library".into(),0,true).unwrap_err();
        assert!(error.contains("5"));
    }
}
