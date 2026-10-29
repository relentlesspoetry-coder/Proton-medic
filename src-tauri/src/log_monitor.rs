//! Polling log monitor: metadata first, bounded analysis only after changes.
//! Existing logs are a read-only baseline. One automatic attempt/game/app run.
use crate::{advisor, diagnostics, repairs, workflow};
use serde::Serialize;
use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::{LazyLock, Mutex}, time::{SystemTime, UNIX_EPOCH}};

type Stamp = Vec<(PathBuf, u64, SystemTime)>;
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Update {
    pub app_id: String, pub library_path: String, pub game_name: String,
    pub state: String, pub message: String, pub codes: Vec<String>,
    pub session: Option<workflow::FixStarted>,
}
struct Entry {
    stamp: Stamp, changed: bool, stable: bool, attempted: bool,
    eligible: bool, reason: String, tool: Option<String>, update: Update,
}
impl Entry {
    fn observe(&mut self, stamp: Stamp) -> bool {
        if self.stamp != stamp {
            self.stamp = stamp; self.changed = true; self.stable = false; true
        } else { self.stable = true; false }
    }
    fn may_apply(&self, enabled: bool, steam_closed: bool) -> bool {
        enabled && steam_closed && self.changed && self.stable && self.eligible && !self.attempted
    }
}
static ENTRIES: LazyLock<Mutex<HashMap<(String,String), Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
fn stamp(home: &std::path::Path, app: &str, library: &str, scan: &crate::scanner::ScanReport) -> Stamp {
    diagnostics::owned_log_candidates(home, app, library, scan).into_iter().filter_map(|path| {
        let meta = std::fs::symlink_metadata(&path).ok()?;
        if !meta.is_file() || meta.file_type().is_symlink() { return None; }
        Some((path, meta.len(), meta.modified().ok()?))
    }).collect()
}
pub fn poll(enabled: bool) -> Result<Vec<Update>, String> {
    crate::game_logging::process_pending();
    let mut entries = ENTRIES.try_lock().map_err(|_| "Log monitor already checking")?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    let scan = crate::inventory::steam();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let closed = !enabled || repairs::require_steam_closed().is_ok();
    let mut applied_this_poll = false;
    for game in &scan.games {
        let key = (game.app_id.clone(), game.library_path.clone());
        let current = stamp(&home, &game.app_id, &game.library_path, &scan);
        let new = !entries.contains_key(&key);
        let entry = entries.entry(key).or_insert_with(|| Entry {
            stamp: current.clone(), changed: false, stable: false, attempted: false, eligible: false, reason: String::new(), tool: None,
            update: Update { app_id: game.app_id.clone(), library_path: game.library_path.clone(), game_name: game.name.clone(),
                state: "watching".into(), message: "Watching for a new game log.".into(), codes: vec![], session: None },
        });
        let modified = entry.observe(current);
        if entry.attempted {
            if entry.update.session.as_ref().is_some_and(|s| !workflow::session_exists(&s.session_id)) {
                entry.update.session = None;
                entry.update.state = "completed".into();
                entry.update.message = "Fix session closed. Automatic changes remain paused for this game until the next app session.".into();
            }
            continue;
        }
        if new || modified {
            let detail = diagnostics::analyze_owned_game(&home, &game.app_id, &game.library_path, &scan);
            let rec = advisor::advise_for_library(&scan, game, Some(&detail));
            let plan = workflow::eligible(game.install_path_exists, &rec.evidence, detail.modified_unix_seconds, now, &rec.status, rec.suggested_tool);
            entry.eligible = plan.can_apply; entry.tool = plan.tool; entry.reason = plan.reason.clone();
            entry.update.codes = plan.evidence;
            entry.update.state = "watching".into();
            entry.update.message = if new { "Existing logs read as a baseline. New failures will be monitored automatically; use Fix & Test to repair an existing finding.".into() }
                else { plan.reason };
        }
        if !entry.changed { continue; }
        if !entry.stable { entry.update.state = "reading".into(); entry.update.message = "Log changed. Waiting for it to settle before considering a fix.".into(); continue; }
        if !entry.eligible { entry.update.state = "review".into(); entry.update.message = entry.reason.clone(); continue; }
        entry.update.state = if !enabled { "detected" } else if !closed { "waiting-for-steam" } else { "queued" }.into();
        entry.update.message = if !enabled { "Supported failure detected. Enable automatic log fixes or use Fix & Test." }
            else if !closed { "Fix queued. Close Steam completely; Proton Medic will back up and apply the supported change automatically." }
            else { "Supported fix queued for application." }.into();
        if !applied_this_poll && entry.may_apply(enabled, closed) {
            // Claim before calling the writer. Errors cannot trigger a retry loop.
            entry.attempted = true; applied_this_poll = true;
            match workflow::start(game.app_id.clone(), game.library_path.clone(), entry.tool.clone().unwrap_or_default(), true) {
                Ok(session) => {
                    entry.update.state = "applied".into();
                    entry.update.message = "Supported Proton selection processed with backup for any change. Open this game's Fix & Test section to launch-test, keep, or roll back. Not yet verified playable.".into();
                    entry.update.session = Some(session);
                }
                Err(error) => { entry.update.state = "needs-attention".into(); entry.update.message = format!("Automatic attempt stopped: {error}"); }
            }
        }
    }
    // Drop non-attempted records for libraries/games no longer present; retain receipts.
    let live: HashSet<_> = scan.games.iter().map(|g| (g.app_id.clone(), g.library_path.clone())).collect();
    entries.retain(|key, e| e.attempted || live.contains(key));
    let mut updates: Vec<_> = entries.values().filter(|e| !e.update.codes.is_empty() || e.changed || e.attempted).map(|e| e.update.clone()).collect();
    for (app_id, library_path) in crate::game_logging::pending()? {
        let game_name = scan.games.iter().find(|g|g.app_id==app_id && g.library_path==library_path)
            .map(|g|g.name.clone()).unwrap_or_else(||format!("Steam app {app_id}"));
        updates.push(Update { app_id, library_path, game_name, state:"logging-queued".into(),
            message:"Log setup is waiting for Steam to close. You can cancel without changing settings.".into(), codes:vec![], session:None });
    }
    updates.sort_by(|a,b| a.game_name.cmp(&b.game_name).then(a.library_path.cmp(&b.library_path)).then(a.app_id.cmp(&b.app_id)));
    Ok(updates)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn entry() -> Entry { Entry { stamp: vec![], changed:false, stable:false, attempted:false, eligible:true, reason:String::new(), tool:Some("P".into()), update:Update { app_id:"1".into(), library_path:"/fixture".into(), game_name:"Fixture".into(), state:"watching".into(), message:String::new(), codes:vec![], session:None } } }
    #[test] fn baseline_and_disabled_monitor_cannot_write() {
        let mut e=entry();e.observe(vec![]);assert!(!e.may_apply(true,true));
        e.changed=true;assert!(!e.may_apply(false,true));
    }
    #[test] fn growing_logs_wait_then_defer_until_steam_closes() {
        let mut e=entry(); let s=vec![(PathBuf::from("log"),10,UNIX_EPOCH)];
        assert!(e.observe(s.clone()));assert!(!e.may_apply(true,true));
        assert!(!e.observe(s));assert!(!e.may_apply(true,false));assert!(e.may_apply(true,true));
    }
    #[test] fn attempt_is_not_rearmed_by_more_log_writes() {
        let mut e=entry();e.attempted=true;
        let s=vec![(PathBuf::from("log"),20,UNIX_EPOCH)];e.observe(s.clone());e.observe(s);
        assert!(!e.may_apply(true,true));
    }
    #[test] fn metadata_detects_new_log_and_ignores_symlinks() {
        let root=std::env::temp_dir().join(format!("pm-monitor-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let library=root.join(".local/share/Steam");
        std::fs::create_dir_all(library.join("steamapps")).unwrap();
        let scan=crate::scanner::scan_home(&root);let library=library.to_str().unwrap();
        assert!(stamp(&root,"123",library,&scan).is_empty());
        let log=root.join("steam-123.log");std::fs::write(&log,"unhandled exception").unwrap();assert_eq!(stamp(&root,"123",library,&scan).len(),1);
        let other=root.join(".var/app/com.valvesoftware.Steam/steam-123.log");
        std::fs::create_dir_all(other.parent().unwrap()).unwrap();std::fs::write(&other,"vk_error_incompatible_driver").unwrap();
        assert_eq!(stamp(&root,"123",library,&scan).len(),1);
        assert_eq!(diagnostics::analyze_owned_game(&root,"123",library,&scan).log_path.as_deref(),log.to_str());
        let first=stamp(&root,"123",library,&scan);std::fs::write(&log,"unhandled exception more evidence").unwrap();assert_ne!(first,stamp(&root,"123",library,&scan));
        std::fs::remove_file(&log).unwrap();std::os::unix::fs::symlink(root.join("other"),&log).unwrap();assert!(stamp(&root,"123",library,&scan).is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
