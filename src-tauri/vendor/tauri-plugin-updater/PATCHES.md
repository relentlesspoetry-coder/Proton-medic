# Proton Medic Linux AppImage patch

Base: the exact crates.io `tauri-plugin-updater` 2.13.3 package.
Original `.crate` SHA256: `855a336eb389d2c1502244fd48fa2d8cb487a430f691bbaed025195f99a04489` (matches Proton Medic's registry Cargo.lock entry).
Original `.cargo_vcs_info.json`, Cargo manifests, and license files are retained.

Focused delta: `src/updater.rs` delegates only its Linux/BSD AppImage install branch to the new private `src/updater/appimage.rs` module. Downloads, signatures, signed versions, public APIs, DEB/RPM, Windows, and macOS behavior are unchanged.

The AppImage installer writes to a private temporary file beside the target, preserves existing permissions, syncs file contents, and atomically renames it over the target. The original stays runnable until commit; interrupted staging cannot truncate or remove it. The installer does not depend on TMPDIR or the cache directory existing, being writable, or sharing the target filesystem. An uncatchable process termination can leave an unused sibling staging file, but not a broken install path. Atomic replacement does not promise recovery from storage-device failure or filesystem power-loss behavior.

Archive updates copy a regular AppImage member into the staging file rather than unpacking paths or archive permissions. Read/CRC errors, missing AppImages, and linked AppImage members fail before replacement. Installing through a symlink deliberately retains prior behavior: replace the link itself, preserve the referent's permissions, and leave the referent untouched.

The private staging-writer seam and subprocess barrier test interruption without adding runtime switches or public APIs. Unit regressions cover raw/archive success, permissions, partial-write failure, precommit original preservation, SIGKILL while staging, invalid temp/cache paths, symlinks, missing/linked/corrupt archive payloads, and temporary-file cleanup after ordinary failures.

Run on Linux with the repository's native prerequisites after wiring the path dependency:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml -p tauri-plugin-updater --lib updater::appimage::tests
```
