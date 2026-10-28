//! Opt-in, same-filesystem archive/restore of two game-generated directories.
//! No deletes, shell, elevation, or free-form filesystem paths from the frontend.
use crate::scanner::{self, Game};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Mutex;

// Serialize write operations across all IPC requests.
static REPAIR_LOCK: Mutex<()> = Mutex::new(());

const VAULT: &str = ".proton-medic-backups";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind { ShaderCache, ProtonPrefix }
impl Kind {
    fn from_id(id: &str) -> Result<Self, String> {
        match id {
            "shader-cache" => Ok(Self::ShaderCache),
            "proton-prefix" => Ok(Self::ProtonPrefix),
            _ => Err("Unknown repair operation".into()),
        }
    }
    fn id(self) -> &'static str { match self { Self::ShaderCache => "shader-cache", Self::ProtonPrefix => "proton-prefix" } }
    fn directory(self) -> &'static str { match self { Self::ShaderCache => "shadercache", Self::ProtonPrefix => "compatdata" } }
    fn label(self) -> &'static str { match self { Self::ShaderCache => "Archive shader cache", Self::ProtonPrefix => "Archive Proton prefix" } }
    fn description(self) -> &'static str { match self {
        Self::ShaderCache => "Move this game's shader cache into a protected backup. Steam may regenerate it; the next launch may stutter while shaders rebuild. This is not a guaranteed crash fix.",
        Self::ProtonPrefix => "Move the ENTIRE Proton compatibility directory into a backup so Steam can rebuild it. This directory MAY CONTAIN LOCAL SAVE GAMES and Windows application settings. Back up or synchronize saves before proceeding. This is an advanced last-resort step.",
    }}
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairPlan {
    pub app_id: String,
    pub library_path: String,
    pub game_name: String,
    pub action: String,
    pub title: String,
    pub explanation: String,
    pub source_path: String,
    pub backup_root: String,
    pub risk: String,
    pub can_apply: bool,
    pub blocker: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairBackup {
    pub name: String,
    pub app_id: String,
    pub library_path: String,
    pub action: String,
    pub backup_path: String,
    pub original_path: String,
    pub restored_target_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOutcome {
    pub message: String,
    pub backup: RepairBackup,
    pub displaced_backup: Option<RepairBackup>,
}

fn identify_game(app_id: &str, library_path: &str) -> Result<Game, String> {
    // Re-scan trusted Steam manifests. Browser-provided paths are never write targets.
    let report = scanner::scan();
    report.games.into_iter().find(|game| game.app_id == app_id && game.library_path == library_path)
        .ok_or_else(|| "Game not present in the current Steam library scan. Rescan and try again.".into())
}

fn path_to_string(path: &Path) -> String { path.to_string_lossy().into_owned() }
fn source_for(game: &Game, kind: Kind) -> PathBuf {
    Path::new(&game.library_path).join("steamapps").join(kind.directory()).join(&game.app_id)
}
fn vault_for(game: &Game) -> PathBuf { Path::new(&game.library_path).join("steamapps").join(VAULT) }
fn no_symlink(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(format!("Refusing symbolic link: {}", path_to_string(path))),
        Ok(meta) => Ok(meta.is_dir()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(format!("Cannot inspect {}: {err}", path_to_string(path))),
    }
}
fn validate_locations(game: &Game, kind: Kind) -> Result<(PathBuf, PathBuf), String> {
    let steamapps = Path::new(&game.library_path).join("steamapps");
    if !no_symlink(&steamapps)? { return Err("Steam apps directory is not a normal directory".into()); }
    let parent = steamapps.join(kind.directory());
    if !no_symlink(&parent)? { return Err(format!("{} is missing or not a directory", path_to_string(&parent))); }
    let vault = vault_for(game);
    // Do not traverse a user-planted backup-root symlink.
    let _ = no_symlink(&vault)?;
    Ok((parent.join(&game.app_id), vault))
}
fn steam_is_running() -> bool {
    let Ok(proc_dir) = fs::read_dir("/proc") else { return true }; // fail closed
    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) { continue; }
        if let Ok(comm) = fs::read_to_string(entry.path().join("comm")) {
            let name = comm.trim().to_ascii_lowercase();
            if matches!(name.as_str(), "steam" | "steamwebhelper" | "steam.sh" | "steam-runtime") { return true; }
        }
    }
    false
}
pub(crate) fn require_steam_closed() -> Result<(), String> {
    if steam_is_running() { Err("Close Steam completely (including its background process) before changing or restoring game files.".into()) }
    else { Ok(()) }
}

pub fn plan(app_id: String, library_path: String, action: String) -> Result<RepairPlan, String> {
    let game = identify_game(&app_id, &library_path)?;
    let kind = Kind::from_id(&action)?;
    let (source, vault) = validate_locations(&game, kind)?;
    let exists = no_symlink(&source)?;
    let blocker = if !exists { Some("Nothing to archive for this game. This directory does not exist.".into()) }
        else if steam_is_running() { Some("Close Steam completely before starting this operation.".into()) }
        else { None };
    Ok(RepairPlan {
        app_id: game.app_id, library_path: game.library_path, game_name: game.name,
        action: kind.id().into(), title: kind.label().into(), explanation: kind.description().into(),
        source_path: path_to_string(&source), backup_root: path_to_string(&vault),
        risk: if kind == Kind::ProtonPrefix { "high" } else { "moderate" }.into(),
        can_apply: blocker.is_none(), blocker,
    })
}

fn create_vault(vault: &Path) -> Result<(), String> {
    if no_symlink(vault)? { return Ok(()); }
    #[cfg(unix)] {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        // No recursive creation: steamapps must already exist.
        match builder.create(vault) {
            Ok(()) => {},
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {},
            Err(err) => return Err(format!("Could not create backup directory: {err}")),
        }
    }
    #[cfg(not(unix))] { fs::create_dir(vault).map_err(|e| e.to_string())?; }
    if !no_symlink(vault)? { return Err("Backup directory is not a normal directory".into()); }
    Ok(())
}
fn name_for(kind: Kind, app_id: &str) -> Result<String, String> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_nanos();
    Ok(format!("{}-{}-{}-{}", kind.id(), app_id, now, std::process::id()))
}
fn backup_record(game: &Game, kind: Kind, name: &str, original_exists: bool) -> RepairBackup {
    RepairBackup { name: name.into(), app_id: game.app_id.clone(), library_path: game.library_path.clone(),
        action: kind.id().into(), backup_path: path_to_string(&vault_for(game).join(name)),
        original_path: path_to_string(&source_for(game, kind)), restored_target_exists: original_exists }
}
fn valid_backup_name(name: &str, kind: Kind, app_id: &str) -> bool {
    let prefix = format!("{}-{}-", kind.id(), app_id);
    let Some(remainder) = name.strip_prefix(&prefix) else { return false };
    let Some((nonce, pid)) = remainder.split_once('-') else { return false };
    !nonce.is_empty() && !pid.is_empty() && nonce.len() <= 40 && pid.len() <= 12
        && nonce.bytes().all(|b| b.is_ascii_digit()) && pid.bytes().all(|b| b.is_ascii_digit())
}
fn archive_path(game: &Game, kind: Kind) -> Result<RepairBackup, String> {
    let (source, vault) = validate_locations(game, kind)?;
    if !no_symlink(&source)? { return Err("This directory no longer exists. Rescan the game.".into()); }
    create_vault(&vault)?;
    for _ in 0..4 {
        let name = name_for(kind, &game.app_id)?;
        let target = vault.join(&name);
        if fs::symlink_metadata(&target).is_ok() { continue; }
        // Single-filesystem atomic rename; no copies or deletes. Fail on cross-device moves.
        fs::rename(&source, &target).map_err(|e| format!("Unable to archive game directory: {e}"))?;
        return Ok(backup_record(game, kind, &name, false));
    }
    Err("Could not allocate a unique backup name".into())
}

pub fn apply(app_id: String, library_path: String, action: String, confirmed: bool) -> Result<RepairOutcome, String> {
    if !confirmed { return Err("Explicit confirmation required".into()); }
    let _guard = REPAIR_LOCK.lock().map_err(|_| "Repair lock is unavailable".to_string())?;
    let game = identify_game(&app_id, &library_path)?;
    let kind = Kind::from_id(&action)?;
    require_steam_closed()?;
    let backup = archive_path(&game, kind)?;
    Ok(RepairOutcome { message: format!("{} archived. Nothing was deleted. You can restore this backup from Fixes.", kind.label()), backup, displaced_backup: None })
}

pub fn list_backups(app_id: String, library_path: String) -> Result<Vec<RepairBackup>, String> {
    let game = identify_game(&app_id, &library_path)?;
    list_game_backups(&game)
}
fn list_game_backups(game: &Game) -> Result<Vec<RepairBackup>, String> {
    let vault = vault_for(game);
    if !no_symlink(&vault)? { return Ok(vec![]); }
    let mut backups = vec![];
    for entry in fs::read_dir(&vault).map_err(|e| e.to_string())?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        for kind in [Kind::ShaderCache, Kind::ProtonPrefix] {
            if valid_backup_name(&name, kind, &game.app_id) && no_symlink(&entry.path())? {
                backups.push(backup_record(game, kind, &name, source_for(game, kind).exists()));
            }
        }
    }
    backups.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(backups)
}

pub fn restore(app_id: String, library_path: String, action: String, backup_name: String, confirmed: bool) -> Result<RepairOutcome, String> {
    if !confirmed { return Err("Explicit confirmation required".into()); }
    let _guard = REPAIR_LOCK.lock().map_err(|_| "Repair lock is unavailable".to_string())?;
    let game = identify_game(&app_id, &library_path)?;
    let kind = Kind::from_id(&action)?;
    require_steam_closed()?;
    restore_for_game(&game, kind, &backup_name)
}

fn restore_for_game(game: &Game, kind: Kind, backup_name: &str) -> Result<RepairOutcome, String> {
    if !valid_backup_name(backup_name, kind, &game.app_id) { return Err("Backup identifier is invalid".into()); }
    let (source, vault) = validate_locations(game, kind)?;
    if !no_symlink(&vault)? { return Err("Backup folder missing".into()); }
    let backup_path = vault.join(&backup_name);
    if !no_symlink(&backup_path)? { return Err("Backup missing or not a regular directory".into()); }
    let mut displaced = None;
    if no_symlink(&source)? {
        // Preserve any regenerated cache/prefix, rather than replacing or deleting it.
        displaced = Some(archive_path(game, kind)?);
    }
    if let Err(err) = fs::rename(&backup_path, &source) {
        // Recovery is best effort; never delete either directory.
        if let Some(ref newer) = displaced {
            let _ = fs::rename(&newer.backup_path, &source);
        }
        return Err(format!("Could not restore backup: {err}. No backup was intentionally deleted."));
    }
    Ok(RepairOutcome {
        message: "Previous game directory restored. Any newer directory was archived separately.".into(),
        backup: backup_record(game, kind, backup_name, true), displaced_backup: displaced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let id = NEXT.fetch_add(1, Ordering::SeqCst);
            let root = std::env::temp_dir().join(format!("proton-medic-repair-{}-{}", std::process::id(), id));
            fs::create_dir_all(root.join("steamapps/compatdata/111/pfx")).unwrap();
            fs::create_dir_all(root.join("steamapps/shadercache/111")).unwrap();
            Self(root)
        }
        fn game(&self) -> Game { Game { app_id: "111".into(), name: "Fixture".into(), library_path: path_to_string(&self.0), install_path: String::new(), install_path_exists: true, manifest_path: String::new(), prefix_path: String::new(), prefix_exists: true, build_id: None, configured_tool: None } }
    }
    impl Drop for TestDir { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    #[test] fn backup_name_validation_rejects_traversal_and_other_game() {
        assert!(valid_backup_name("proton-prefix-111-12345-99", Kind::ProtonPrefix, "111"));
        assert!(!valid_backup_name("../../etc", Kind::ProtonPrefix, "111"));
        assert!(!valid_backup_name("shader-cache-222-123-1", Kind::ShaderCache, "111"));
    }
    #[test] fn archive_preserves_data_and_location() {
        let t = TestDir::new(); let game = t.game();
        fs::write(t.0.join("steamapps/compatdata/111/pfx/save.dat"), "important save").unwrap();
        let backup = archive_path(&game, Kind::ProtonPrefix).unwrap();
        assert!(!t.0.join("steamapps/compatdata/111").exists());
        assert_eq!(fs::read_to_string(Path::new(&backup.backup_path).join("pfx/save.dat")).unwrap(), "important save");
        assert_eq!(list_game_backups(&game).unwrap().len(), 1);
    }
    #[test] fn symlink_source_rejected() {
        let t = TestDir::new(); let game = t.game();
        fs::remove_dir_all(t.0.join("steamapps/shadercache/111")).unwrap();
        #[cfg(unix)] {
            std::os::unix::fs::symlink("/tmp", t.0.join("steamapps/shadercache/111")).unwrap();
            assert!(archive_path(&game, Kind::ShaderCache).is_err());
        }
    }
    #[test] fn reject_vault_symlink() {
        let t = TestDir::new(); let game = t.game();
        #[cfg(unix)] {
            std::os::unix::fs::symlink("/tmp", t.0.join("steamapps/.proton-medic-backups")).unwrap();
            assert!(archive_path(&game, Kind::ShaderCache).is_err());
        }
    }
    #[test] fn restoring_archives_newer_data_without_overwriting_either() {
        let t = TestDir::new(); let game = t.game();
        let saved = t.0.join("steamapps/compatdata/111/pfx/save.dat");
        fs::write(&saved, "old progress").unwrap();
        let previous = archive_path(&game, Kind::ProtonPrefix).unwrap();
        fs::create_dir_all(t.0.join("steamapps/compatdata/111/pfx")).unwrap();
        fs::write(&saved, "new progress").unwrap();
        let restored = restore_for_game(&game, Kind::ProtonPrefix, &previous.name).unwrap();
        assert_eq!(fs::read_to_string(&saved).unwrap(), "old progress");
        let newer = restored.displaced_backup.unwrap();
        assert_eq!(fs::read_to_string(Path::new(&newer.backup_path).join("pfx/save.dat")).unwrap(), "new progress");
        assert!(!Path::new(&previous.backup_path).exists());
    }
    #[test] fn restore_rejects_borrowed_or_malicious_backup_name() {
        let t = TestDir::new(); let game = t.game();
        assert!(restore_for_game(&game, Kind::ProtonPrefix, "../../etc").is_err());
        assert!(restore_for_game(&game, Kind::ProtonPrefix, "shader-cache-111-999-1").is_err());
    }

}
