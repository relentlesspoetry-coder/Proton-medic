//! Advisory locks shared by GUI instances and desktop shortcut supervisors.
use std::{fs::{self, File, OpenOptions}, os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt}, path::{Path, PathBuf}};

fn directory(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new(); builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => {},
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {},
        Err(e) => return Err(e.to_string()),
    }
    let meta = fs::symlink_metadata(path).map_err(|e|e.to_string())?;
    if !meta.is_dir() || meta.file_type().is_symlink() || meta.uid() != unsafe { libc::geteuid() } {
        return Err("Application lock directory must be owned by this user and cannot be a symbolic link".into());
    }
    Ok(())
}
fn update_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p|p.is_absolute())
        .or_else(||std::env::var_os("HOME").map(|h|PathBuf::from(h).join(".local/share")))
        .filter(|p|p.is_absolute()).ok_or("Application data directory unavailable")?;
    fs::create_dir_all(&base).map_err(|e|e.to_string())?;
    let app = fs::canonicalize(base).map_err(|e|e.to_string())?.join("proton-medic");
    directory(&app)?;
    let locks = app.join("locks"); directory(&locks)?;
    Ok(locks.join("activity.lock"))
}
fn lock(path: &Path, shared: bool) -> Result<File, String> {
    let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK).open(path).map_err(|e|e.to_string())?;
    let meta = file.metadata().map_err(|e|e.to_string())?;
    if !meta.is_file() || meta.uid() != unsafe { libc::geteuid() } || meta.nlink() != 1 {
        return Err("Expected a user-owned regular lock file without hard links".into());
    }
    if shared { file.try_lock_shared() } else { file.try_lock() }
        .map_err(|_|"Another Proton Medic operation holds this lock".to_owned())?;
    Ok(file)
}
pub fn activity() -> Result<File, String> {
    lock(&update_path()?, true).map_err(|e|format!("Cannot start while an app update is active: {e}"))
}
pub fn update() -> Result<File, String> {
    lock(&update_path()?, false).map_err(|e|format!("Finish games, installers, downloads, and repairs in all Proton Medic windows before updating: {e}"))
}
// Lock a stable sibling, not the configuration inode replaced by atomic writes.
pub fn config(path: &Path) -> Result<File, String> {
    let parent = fs::canonicalize(path.parent().ok_or("Configuration directory unavailable")?).map_err(|e|e.to_string())?;
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().ok_or("Configuration filename unavailable")?);
    name.push(".proton-medic.lock");
    lock(&parent.join(name), false)
        .map_err(|e|format!("Another Proton Medic instance is changing this Steam configuration. Retry after it finishes: {e}"))
}
pub fn prefix(prefix: &Path) -> Result<File, String> {
    let prefix = fs::canonicalize(prefix).map_err(|e|e.to_string())?;
    lock(&prefix.join(".proton-medic.lock"), false)
        .map_err(|e|format!("This Windows prefix is in use by another Proton Medic game or installer: {e}"))
}

#[cfg(test)] mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture { fn new() -> Self {
        let path = std::env::temp_dir().join(format!("pm-coordination-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir(&path).unwrap(); Self(path)
    } }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    #[test] fn readers_coexist_but_exclude_updates_until_every_reader_finishes() {
        let f=Fixture::new();let path=f.0.join("activity.lock");
        let a=lock(&path,true).unwrap();let b=lock(&path,true).unwrap();
        assert!(lock(&path,false).is_err());drop(a);assert!(lock(&path,false).is_err());drop(b);
        let update=lock(&path,false).unwrap();assert!(lock(&path,true).is_err());assert!(lock(&path,false).is_err());
        drop(update);assert!(lock(&path,true).is_ok());
    }
    #[test] fn prefix_aliases_cannot_bypass_exclusive_ownership() {
        let f=Fixture::new();let prefix_path=f.0.join("pfx");fs::create_dir(&prefix_path).unwrap();
        let alias=f.0.join("alias");std::os::unix::fs::symlink(&prefix_path,&alias).unwrap();
        let first=prefix(&prefix_path).unwrap();assert!(prefix(&alias).is_err());drop(first);assert!(prefix(&alias).is_ok());
    }
    #[test] fn existing_symlinks_and_hard_links_are_never_used_as_locks() {
        let f=Fixture::new();let target=f.0.join("keep");fs::write(&target,b"keep").unwrap();
        let link=f.0.join("link");std::os::unix::fs::symlink(&target,&link).unwrap();assert!(lock(&link,false).is_err());
        let hard=f.0.join("hard");fs::hard_link(&target,&hard).unwrap();assert!(lock(&hard,false).is_err());
        assert_eq!(fs::read(target).unwrap(),b"keep");
    }
}
