//! One evidence-backed change and one observed launch; never a repair loop.
use crate::{advisor, auto_select, diagnostics, launch_test, scanner};
use serde::Serialize;
use std::{collections::HashMap, sync::{Mutex, LazyLock}, time::{SystemTime, UNIX_EPOCH}};

#[derive(Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct FixPlan {
    pub can_apply: bool,
    pub reason: String,
    pub tool: Option<String>,
    pub evidence: Vec<String>,
}

pub(crate) fn eligible(installed: bool, codes: &[String], modified: Option<u64>, now: u64,
            recommendation: &str, tool: Option<String>) -> FixPlan {
    let has = |code: &str| codes.iter().any(|c| c == code);
    let reason = if !installed { Some("The game folder is unavailable. Reconnect its drive and scan again.") }
        else if has("anticheat-blocked") { Some("The log reports an unsupported anti-cheat platform. Proton Medic will not bypass anti-cheat.") }
        else if has("vulkan-init") || has("missing-shared-lib") { Some("This evidence points to a system dependency or graphics problem. A Proton switch is not the supported repair for this finding yet.") }
        else if !modified.is_some_and(|t| t <= now + 300 && now.saturating_sub(t) <= 48 * 3600) { Some("Create a fresh Proton log before applying a fix. Diagnose this game to prepare logging automatically, then approve a test launch.") }
        else if !has("wine-exception") && !has("vulkan-device-lost") { Some("No supported automatic repair matches this evidence yet. Existing manual diagnostics and repair tools remain available.") }
        else if recommendation != "try-version" || tool.is_none() { Some("No different installed Proton candidate is available for this repair.") }
        else { None };
    FixPlan { can_apply: reason.is_none(), reason: reason.unwrap_or("Try one installed Proton alternative, preserve the current configuration, and observe a new launch. Playability requires your confirmation.").into(), tool, evidence: codes.to_vec() }
}

pub fn plan(app_id: &str, library: &str) -> Result<FixPlan, String> {
    let scan = scanner::scan();
    let game = scan.games.iter().find(|g| g.app_id == app_id && g.library_path == library)
        .ok_or("Game is not in the current Steam scan")?;
    let report = diagnostics::diagnose(&scan);
    let detail = report.games.iter().find(|g| g.app_id == app_id && g.library_path == library)
        .ok_or("No diagnostics available")?;
    let advice = advisor::recommend(&scan, &report);
    let rec = advice.games.iter().find(|g| g.app_id == app_id && g.library_path == library)
        .ok_or("No Proton recommendation available")?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    Ok(eligible(game.install_path_exists, &rec.evidence, detail.modified_unix_seconds, now,
                &rec.status, rec.suggested_tool.clone()))
}

struct Session { app_id: String, library: String, undo: Option<auto_select::ConfigUndo>, attempted: bool }
static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
pub(crate) fn require_update_idle() -> Result<(),String> {
    if !SESSIONS.try_lock().map_err(|_|"Wait for the current repair")?.is_empty() {
        return Err("Finish or roll back open Fix & Test sessions before updating.".into());
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct FixStarted { pub session_id: String, pub selection: auto_select::AutoSelection }

pub fn start(app_id: String, library: String, expected_tool: String, confirmed: bool) -> Result<FixStarted, String> {
    if !confirmed { return Err("Approve the configuration change before applying the fix".into()); }
    let mut sessions = SESSIONS.lock().map_err(|_| "Fix session lock unavailable")?;
    if sessions.len() >= 32 { return Err("Finish or roll back existing fix sessions before starting more".into()); }
    if sessions.values().any(|s| s.app_id == app_id && s.library == library) {
        return Err("A fix session already exists for this game. Finish it before trying another change.".into());
    }
    let fresh = plan(&app_id, &library)?;
    if !fresh.can_apply { return Err(fresh.reason); }
    if fresh.tool.as_deref() != Some(expected_tool.as_str()) { return Err("Recommendation changed. Diagnose again and review the new fix.".into()); }
    let (selection, undo) = auto_select::select_tracked(app_id.clone(), library.clone(), true, Some(&expected_tool))?;
    let id = format!("{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos());
    sessions.insert(id.clone(), Session { app_id, library, undo, attempted: false });
    Ok(FixStarted { session_id: id, selection })
}

pub fn test(id: &str, confirmed: bool) -> Result<launch_test::LaunchResult, String> {
    if !confirmed { return Err("Approve opening the game before launch testing".into()); }
    let mut sessions = SESSIONS.lock().map_err(|_| "Fix session lock unavailable")?;
    let session = sessions.get_mut(id).ok_or("Fix session unavailable; the backup remains on disk")?;
    observe_once(session, |app_id, library| launch_test::test_game(app_id.into(), library.into(), 10, true))
}

fn observe_once(session: &mut Session, launch: impl FnOnce(&str, &str) -> Result<launch_test::LaunchResult, String>) -> Result<launch_test::LaunchResult, String> {
    if session.attempted { return Err("This fix already had its launch attempt. Confirm the result or roll back.".into()); }
    session.attempted = true;
    launch(&session.app_id, &session.library)
}

pub fn rollback(id: &str, confirmed: bool) -> Result<String, String> {
    if !confirmed { return Err("Approve rollback before restoring configuration".into()); }
    let mut sessions = SESSIONS.lock().map_err(|_| "Fix session lock unavailable")?;
    let session = sessions.get(id).ok_or("Fix session unavailable; use the saved configuration backup")?;
    let message = if let Some(undo) = &session.undo {
        let backup = auto_select::rollback(undo, true)?;
        format!("Previous Steam configuration restored. The replaced configuration was also backed up: {backup}")
    } else { "No configuration change was made; nothing to restore.".into() };
    sessions.remove(id);
    Ok(message)
}

pub fn finish(id: &str, confirmed: bool) -> Result<(), String> {
    if !confirmed { return Err("Confirm keeping the selection before finishing".into()); }
    SESSIONS.lock().map_err(|_| "Fix session lock unavailable")?.remove(id)
        .ok_or("Fix session unavailable")?;
    Ok(())
}

pub(crate) fn session_exists(id: &str) -> bool {
    SESSIONS.lock().map(|sessions| sessions.contains_key(id)).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check(codes: &[&str], date: Option<u64>) -> FixPlan {
        eligible(true, &codes.iter().map(|s| (*s).into()).collect::<Vec<_>>(), date, 200_000,
                 "try-version", Some("Proton Experimental".into()))
    }
    #[test] fn matches_only_supported_repairs() {
        for code in ["wine-exception", "vulkan-device-lost"] { assert!(check(&[code], Some(200_000)).can_apply); }
        for code in ["memory", "wine-module", "anticheat"] { assert!(!check(&[code], Some(200_000)).can_apply); }
    }
    #[test] fn system_and_anticheat_blockers_override_switch() {
        for code in ["vulkan-init", "missing-shared-lib", "anticheat-blocked"] {
            assert!(!check(&["wine-exception", code], Some(200_000)).can_apply);
        }
        assert!(check(&["wine-exception", "anticheat"], Some(200_000)).can_apply);
    }
    #[test] fn rejects_missing_stale_and_future_evidence() {
        for date in [None, Some(1), Some(200_301)] { assert!(!check(&["wine-exception"], date).can_apply); }
    }
    #[test] fn requires_installed_game_and_alternative() {
        assert!(!eligible(false, &["wine-exception".into()], Some(200_000), 200_000, "try-version", Some("P".into())).can_apply);
        assert!(!eligible(true, &["wine-exception".into()], Some(200_000), 200_000, "keep-current", Some("P".into())).can_apply);
    }
    #[test] fn all_mutations_require_consent_before_io() {
        assert!(start("1".into(), "/none".into(), "P".into(), false).unwrap_err().contains("Approve"));
        assert!(test("none", false).unwrap_err().contains("Approve"));
        assert!(rollback("none", false).unwrap_err().contains("Approve"));
        assert!(finish("none", false).unwrap_err().contains("Confirm"));
    }
    #[test] fn failed_launch_is_not_retried_automatically() {
        let mut session = Session { app_id: "123".into(), library: "/fixture".into(), undo: None, attempted: false };
        let calls = std::cell::Cell::new(0);
        let first = observe_once(&mut session, |app, library| {
            assert_eq!(app,"123"); assert_eq!(library,"/fixture");
            calls.set(calls.get()+1); Err("fixture launch failed".into())
        });
        assert_eq!(first.unwrap_err(),"fixture launch failed");
        assert!(observe_once(&mut session, |_,_| { calls.set(calls.get()+1); Err("unexpected retry".into()) }).is_err());
        assert_eq!(calls.get(),1);
    }

}
