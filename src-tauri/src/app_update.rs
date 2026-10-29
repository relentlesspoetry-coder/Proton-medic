//! Serialize updates against game launches and configuration changes.
use std::{path::{Path, PathBuf}, process::Stdio, sync::{OnceLock, RwLock, RwLockReadGuard}};
static UPDATING: RwLock<bool> = RwLock::new(false);
static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();
pub fn initialize() { if let Ok(path) = crate::portable::executable() { let _ = EXECUTABLE.set(path); } }
pub fn activity() -> Result<RwLockReadGuard<'static, bool>, String> { activity_in(&UPDATING) }
fn activity_in(state: &RwLock<bool>) -> Result<RwLockReadGuard<'_, bool>, String> {
    let guard = state.read().map_err(|_| "Update state unavailable")?;
    if *guard { return Err("An app update is in progress. Wait for Proton Medic to reopen.".into()); }
    Ok(guard)
}
pub fn begin() -> Result<(), String> {
    begin_in(&UPDATING, || {
        crate::windows_games::require_update_idle()?;
        crate::workflow::require_update_idle()?;
        crate::game_logging::require_update_idle()
    })
}
fn begin_in(state: &RwLock<bool>, idle: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let mut updating = state.try_write().map_err(|_| "Finish the current operation before updating")?;
    if *updating { return Err("An app update is already in progress".into()); }
    idle()?;
    *updating = true; Ok(())
}
pub fn cancel() -> Result<(), String> {
    *UPDATING.write().map_err(|_| "Update state unavailable")? = false; Ok(())
}
fn validate_version(value: &str) -> Result<(), String> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty() || p.len() > 10 || !p.bytes().all(|b| b.is_ascii_digit())) {
        return Err("Invalid installed update version".into());
    }
    Ok(())
}
fn verify_installed(path: &Path, version: &str) -> Result<(), String> {
    validate_version(version)?;
    let output = crate::font_runtime::child_command(path).arg("--proton-medic-version").stdin(Stdio::null()).output()
        .map_err(|e| format!("Could not open the updated program: {e}"))?;
    if !output.status.success() || output.stdout != format!("{version}\n").as_bytes() {
        return Err("The installed program does not match the downloaded version. Keep this window open and retry after installation completes.".into());
    }
    Ok(())
}
fn launch_installed(path: &Path, version: &str) -> Result<std::process::Child, String> {
    verify_installed(path, version)?;
    use std::os::unix::process::CommandExt;
    crate::font_runtime::child_command(path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .process_group(0).spawn().map_err(|e| format!("Update installed, but relaunch failed: {e}"))
}
pub fn restart(app: tauri::AppHandle, version: String) -> Result<(), String> {
    let updating = UPDATING.read().map_err(|_| "Update state unavailable")?;
    if !*updating { return Err("No app update is awaiting relaunch".into()); }
    let path = EXECUTABLE.get().ok_or("Installed program path unavailable")?;
    launch_installed(path, &version)?;
    app.exit(0); Ok(())
}
#[cfg(test)] mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test] fn rejects_invalid_versions() {
        for v in ["", "1.2", "1.2.3.4", "1.2.3;sh", "../1", "1.2.-3"] { assert!(validate_version(v).is_err()); }
        assert!(validate_version("0.6.11").is_ok());
    }
    #[test] fn installed_version_probe_checks_exit_and_exact_version() {
        let path = std::env::temp_dir().join(format!("pm-update-version-{}",std::process::id()));
        std::fs::write(&path, "#!/bin/sh\n[ \"$1\" = --proton-medic-version ] || exit 2\nprintf '0.6.11\\n'\n").unwrap();
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(verify_installed(&path,"0.6.11").is_ok()); assert!(verify_installed(&path,"0.6.12").is_err());
        std::fs::remove_file(&path).unwrap(); assert!(verify_installed(&path,"0.6.11").is_err());
    }
    #[test] fn relaunch_opens_replacement_at_the_saved_install_path() {
        let dir=std::env::temp_dir().join(format!("pm-relaunch-{}",std::process::id()));std::fs::create_dir(&dir).unwrap();
        let path=dir.join("proton-medic");let receipt=dir.join("started");
        std::fs::write(&path,"old executable").unwrap();std::fs::rename(&path,dir.join("previous")).unwrap();
        std::fs::write(&path,format!("#!/bin/sh\nif [ \"$1\" = --proton-medic-version ]; then printf '0.6.12\\n'; else touch '{}'; fi\n",receipt.display())).unwrap();
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(launch_installed(&path,"0.6.11").is_err());assert!(!receipt.exists());
        assert!(launch_installed(&path,"0.6.12").unwrap().wait().unwrap().success());assert!(receipt.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test] fn update_lease_blocks_new_work_and_requires_running_work_to_finish() {
        let state=RwLock::new(false);let work=activity_in(&state).unwrap();
        assert!(begin_in(&state,||Ok(())).is_err());drop(work);
        assert!(begin_in(&state,||Err("Installer running".into())).is_err());
        assert!(activity_in(&state).is_ok());
        begin_in(&state,||Ok(())).unwrap();assert!(activity_in(&state).is_err());
        assert!(begin_in(&state,||Ok(())).is_err());
        *state.write().unwrap()=false;assert!(activity_in(&state).is_ok());
    }

}
