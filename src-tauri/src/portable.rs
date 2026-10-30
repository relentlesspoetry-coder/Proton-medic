//! Stable launch paths and host subprocess isolation for portable bundles.
use std::{ffi::{OsStr, OsString}, path::{Path, PathBuf}, process::Command};

pub fn executable() -> Result<PathBuf, String> {
    select_executable(std::env::var_os("APPIMAGE"), std::env::current_exe().map_err(|e| e.to_string())?)
}

fn select_executable(appimage: Option<OsString>, native: PathBuf) -> Result<PathBuf, String> {
    match appimage {
        Some(value) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() || !path.is_file() {
                return Err("The portable application has moved. Open it from its permanent location before updating or creating shortcuts.".into());
            }
            Ok(path)
        }
        None => Ok(native),
    }
}

// Keep user paths, but never pass the bundle's private libraries, Python modules,
// GTK plugins or WebKit helpers to Steam, Wine, diagnostics, or the next AppImage.
fn clean_value(value: &OsStr, appdir: &Path) -> Option<OsString> {
    let paths: Vec<_> = std::env::split_paths(value).filter(|p| !p.starts_with(appdir)).collect();
    if paths.is_empty() { None } else { std::env::join_paths(paths).ok() }
}

fn clean_preload(value: &OsStr, appdir: &Path) -> Option<OsString> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let mut cleaned = Vec::new();
    // The dynamic loader accepts spaces and colons, without quoting or escaping.
    for entry in value.as_bytes().split(|byte| *byte == b' ' || *byte == b':') {
        if entry.is_empty() || Path::new(OsStr::from_bytes(entry)).starts_with(appdir) { continue; }
        if !cleaned.is_empty() { cleaned.push(b':'); }
        cleaned.extend_from_slice(entry);
    }
    if cleaned.is_empty() { None } else { Some(OsString::from_vec(cleaned)) }
}

pub fn host_environment(command: &mut Command) {
    let Some(appdir) = std::env::var_os("APPDIR").map(PathBuf::from).filter(|p| p.is_absolute()) else { return; };
    for key in ["PATH", "LD_LIBRARY_PATH", "LD_PRELOAD", "PYTHONHOME", "PYTHONPATH",
        "XDG_DATA_DIRS", "GSETTINGS_SCHEMA_DIR", "GI_TYPELIB_PATH", "GIO_MODULE_DIR",
        "GTK_DATA_PREFIX", "GTK_EXE_PREFIX", "GTK_PATH", "GTK_IM_MODULE_FILE",
        "GDK_PIXBUF_MODULE_FILE", "GDK_PIXBUF_MODULEDIR", "GST_PLUGIN_PATH",
        "GST_PLUGIN_SYSTEM_PATH", "GST_PLUGIN_SCANNER", "WEBKIT_EXEC_PATH",
        "WEBKIT_INJECTED_BUNDLE_PATH", "FONTCONFIG_PATH", "FONTCONFIG_FILE"] {
        let value = match command.get_envs().find(|(name, _)| *name == key) {
            Some((_, value)) => value.map(OsStr::to_owned),
            None => std::env::var_os(key),
        };
        if let Some(value) = value {
            let cleaned = if key == "LD_PRELOAD" { clean_preload(&value, &appdir) } else { clean_value(&value, &appdir) };
            match cleaned {
                Some(value) => { command.env(key, value); }
                None => { command.env_remove(key); }
            }
        }
    }
    for key in ["APPDIR", "APPIMAGE", "ARGV0", "OWD"] { command.env_remove(key); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preload_filters_spaces_and_colons_without_reordering_host_libraries() {
        let dir = Path::new("/tmp/.mount_pm");
        let value = OsStr::new(" /host/first.so /tmp/.mount_pm/lib/private.so:/host/second.so::/tmp/.mount_pm-other/lib.so ");
        assert_eq!(clean_preload(value, dir), Some(OsString::from("/host/first.so:/host/second.so:/tmp/.mount_pm-other/lib.so")));
        assert_eq!(clean_preload(OsStr::new("/tmp/.mount_pm/private.so /host/first.so"), dir), Some(OsString::from("/host/first.so")));
        assert_eq!(clean_preload(OsStr::new(" :/tmp/.mount_pm/private.so:: "), dir), None);
        assert_eq!(clean_preload(OsStr::new(" : "), dir), None);
    }
    #[test]
    fn preload_preserves_non_utf8_bytes_and_nonseparator_whitespace() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let dir = Path::new("/tmp/.mount_pm");
        let value = OsStr::from_bytes(b"/host/\xff.so /tmp/.mount_pm/private.so:/host/tab\tname.so");
        assert_eq!(clean_preload(value, dir), Some(OsString::from_vec(b"/host/\xff.so:/host/tab\tname.so".to_vec())));
    }
    #[test]
    fn ordinary_path_values_keep_spaces_inside_entries() {
        let dir = Path::new("/tmp/.mount_pm");
        assert_eq!(clean_value(OsStr::new("/home/deck/My Tools:/tmp/.mount_pm/bin:/usr/bin"), dir),
            Some(OsString::from("/home/deck/My Tools:/usr/bin")));
    }
    #[test]
    fn host_paths_survive_without_bundle_paths_or_similar_prefixes() {
        let dir = Path::new("/tmp/.mount_pm");
        assert_eq!(clean_value(OsStr::new("/tmp/.mount_pm/usr/lib:/usr/lib:/tmp/.mount_pm-other/lib"), dir),
            Some(OsString::from("/usr/lib:/tmp/.mount_pm-other/lib")));
        assert_eq!(clean_value(OsStr::new("/tmp/.mount_pm/schema"), dir), None);
        assert_eq!(clean_value(OsStr::new("/home/deck/custom"), dir), Some(OsString::from("/home/deck/custom")));
    }
    #[test]
    fn portable_launch_path_survives_mount_replacement() {
        let native = PathBuf::from("/tmp/.mount_pm/usr/bin/proton-medic");
        let file = std::env::current_exe().unwrap();
        assert_eq!(select_executable(Some(file.clone().into_os_string()), native.clone()).unwrap(), file);
        assert_eq!(select_executable(None, native.clone()).unwrap(), native);
        assert!(select_executable(Some("relative.AppImage".into()), native.clone()).is_err());
        assert!(select_executable(Some("/missing/proton-medic.AppImage".into()), native).is_err());
    }
}
