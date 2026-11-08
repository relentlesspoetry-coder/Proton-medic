// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Linux AppImage replacement: never remove the running image before staging succeeds.
use std::{fs::File, io::Write, path::Path};

use super::{Error, Result};

pub(super) fn install(target: &Path, bytes: &[u8]) -> Result<()> {
    stage_and_replace(target, |file| write_payload(file, bytes))
}

// The private writer seam permits failure/interruption tests without production
// environment switches, callbacks in the public API, or special update payloads.
fn stage_and_replace(
    target: &Path,
    write_payload: impl FnOnce(&mut File) -> Result<()>,
) -> Result<()> {
    // metadata follows a symlink to obtain the existing executable's permissions.
    // persist replaces the path itself, matching the old installer's behavior:
    // installing through a symlink replaces the link, never its referent.
    let permissions = target.metadata()?.permissions();
    let parent = target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut staged = tempfile::Builder::new()
        .prefix(".tauri-appimage-")
        .tempfile_in(parent)?;
    write_payload(staged.as_file_mut())?;
    staged.as_file().set_permissions(permissions)?;
    staged.as_file().sync_all()?;

    // Same-directory rename is atomic. A write, chmod, sync, or rename failure
    // drops only the staging file; the original has never been moved away.
    // An interrupted process may leave a staging file, but the install path is
    // always a complete old or new image. Do not use remove_file + persist.
    staged.persist(target).map_err(|error| Error::Io(error.error))?;
    Ok(())
}

fn write_payload(file: &mut File, bytes: &[u8]) -> Result<()> {
    #[cfg(feature = "zip")]
    if infer::archive::is_gz(bytes) {
        use std::io::{self, Cursor};
        let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        let mut found = false;
        for entry in archive.entries()? {
            let mut entry = entry?;
            if !found && entry.path()?.extension() == Some(std::ffi::OsStr::new("AppImage")) {
                // Copy bytes, rather than unpacking paths or archive permissions.
                // A link is not an executable payload and must not replace the app.
                if !entry.header().entry_type().is_file() {
                    return Err(Error::InvalidUpdaterFormat);
                }
                io::copy(&mut entry, file)?;
                found = true;
            }
        }
        // Tar's end marker can precede gzip's trailer. Validate the remainder
        // (including CRC/truncation errors) before committing the staged image.
        io::copy(&mut archive.into_inner(), &mut io::sink())?;
        return if found { Ok(()) } else { Err(Error::BinaryNotFoundInArchive) };
    }
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::{symlink, PermissionsExt}, process::{Command, Stdio}, time::{Duration, Instant}};

    const OLD: &[u8] = b"#!/bin/sh\nprintf 'old\\n'\n";
    const NEW: &[u8] = b"#!/bin/sh\nprintf 'new\\n'\n";

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("Proton Medic.AppImage");
        fs::write(&target, OLD).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o751)).unwrap();
        (dir, target)
    }

    #[test]
    fn raw_install_replaces_bytes_and_preserves_permissions() {
        let (dir, target) = fixture();
        install(&target, NEW).unwrap();
        assert_eq!(fs::read(&target).unwrap(), NEW);
        assert_eq!(target.metadata().unwrap().permissions().mode() & 0o777, 0o751);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_partial_write_preserves_original_and_cleans_staging() {
        let (dir, target) = fixture();
        let error = stage_and_replace(&target, |file| {
            file.write_all(b"partial replacement")?;
            assert_eq!(fs::read(&target).unwrap(), OLD);
            Err(std::io::Error::from_raw_os_error(28).into())
        }).unwrap_err();
        assert!(matches!(error, Error::Io(_)));
        assert_eq!(fs::read(&target).unwrap(), OLD);
        assert_eq!(target.metadata().unwrap().permissions().mode() & 0o777, 0o751);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn rename_failure_preserves_target_and_cleans_staging() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("existing-directory");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), OLD).unwrap();
        assert!(install(&target, NEW).is_err());
        assert_eq!(fs::read(target.join("keep")).unwrap(), OLD);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn successful_writer_observes_old_image_until_commit() {
        let (_dir, target) = fixture();
        stage_and_replace(&target, |file| {
            file.write_all(NEW)?;
            file.sync_all()?;
            assert_eq!(fs::read(&target).unwrap(), OLD);
            Ok(())
        }).unwrap();
        assert_eq!(fs::read(&target).unwrap(), NEW);
    }

    #[test]
    fn symlink_install_replaces_link_without_modifying_referent() {
        let (dir, original) = fixture();
        let target = dir.path().join("launcher.AppImage");
        symlink(&original, &target).unwrap();
        install(&target, NEW).unwrap();
        assert!(!target.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&target).unwrap(), NEW);
        assert_eq!(fs::read(&original).unwrap(), OLD);
        assert_eq!(target.metadata().unwrap().permissions().mode() & 0o777, 0o751);
    }

    // Run in a child test process so environment overrides and SIGKILL cannot
    // affect other tests. The barrier is entirely test-only: the private writer
    // notifies its parent after a partial staging write and blocks until killed.
    #[test]
    fn subprocess_fixture() {
        let Some(target) = std::env::var_os("PM_UPDATER_TEST_TARGET") else { return; };
        let target = Path::new(&target);
        if let Some(ready) = std::env::var_os("PM_UPDATER_TEST_READY") {
            stage_and_replace(target, |file| {
                file.write_all(b"partial replacement")?;
                file.sync_all()?;
                fs::write(ready, b"ready")?;
                loop { std::thread::sleep(Duration::from_millis(50)); }
            }).unwrap();
        } else {
            install(target, NEW).unwrap();
        }
    }

    fn child(target: &Path) -> Command {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "updater::appimage::tests::subprocess_fixture", "--nocapture"])
            .env("PM_UPDATER_TEST_TARGET", target)
            .env_remove("PM_UPDATER_TEST_READY")
            .stdout(Stdio::null()).stderr(Stdio::null());
        command
    }

    #[test]
    fn unusable_temp_and_cache_locations_do_not_block_sibling_staging() {
        let (dir, target) = fixture();
        let invalid = dir.path().join("nonexistent").join("cache");
        let status = child(&target).env("TMPDIR", &invalid).env("TMP", &invalid)
            .env("TEMP", &invalid).env("XDG_CACHE_HOME", &invalid).status().unwrap();
        assert!(status.success());
        assert_eq!(fs::read(&target).unwrap(), NEW);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn interrupted_staging_keeps_original_runnable() {
        let (dir, target) = fixture();
        let ready = dir.path().join("ready");
        let mut process = child(&target).env("PM_UPDATER_TEST_READY", &ready).spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() && Instant::now() < deadline {
            if let Some(status) = process.try_wait().unwrap() {
                panic!("fixture exited before staging barrier: {status}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Reap even on a timeout rather than leaving a blocked fixture behind.
        process.kill().unwrap();
        process.wait().unwrap();
        assert!(ready.exists(), "fixture did not reach staging barrier");
        assert_eq!(fs::read(&target).unwrap(), OLD);
        assert_eq!(Command::new(&target).output().unwrap().stdout, b"old\n");
    }

    #[cfg(feature = "zip")]
    fn archive_payload(name: &str, kind: tar::EntryType, contents: &[u8]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(kind);
        header.set_mode(0o600); // Existing executable mode must win.
        header.set_size(contents.len() as u64);
        header.set_cksum();
        archive.append_data(&mut header, name, contents).unwrap();
        archive.into_inner().unwrap().finish().unwrap()
    }

    #[cfg(feature = "zip")]
    #[test]
    fn archive_install_copies_payload_and_preserves_existing_mode() {
        let (_dir, target) = fixture();
        install(&target, &archive_payload("nested/new.AppImage", tar::EntryType::Regular, NEW)).unwrap();
        assert_eq!(fs::read(&target).unwrap(), NEW);
        assert_eq!(target.metadata().unwrap().permissions().mode() & 0o777, 0o751);
    }

    #[cfg(feature = "zip")]
    #[test]
    fn missing_link_and_corrupt_archive_preserve_original() {
        let (dir, target) = fixture();
        let mut corrupt = archive_payload("new.AppImage", tar::EntryType::Regular, NEW);
        let crc_offset = corrupt.len() - 8;
        corrupt[crc_offset] ^= 0xff;
        for bytes in [
            archive_payload("readme.txt", tar::EntryType::Regular, NEW),
            archive_payload("new.AppImage", tar::EntryType::Symlink, b""),
            corrupt,
            vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 3],
        ] {
            assert!(install(&target, &bytes).is_err());
            assert_eq!(fs::read(&target).unwrap(), OLD);
            assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
        }
    }
}
