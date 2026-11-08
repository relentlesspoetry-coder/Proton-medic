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

## Bounded metadata retries

`src/updater/metadata.rs` retries only the release-metadata GET, at most three
attempts, with 500 ms and 1 s waits. All attempts and waits share the caller's
check timeout (20 seconds in Proton Medic), including a reduced request timeout
on later attempts. A transient HTTP 408/500/502/503/504 or typed reqwest timeout
is eligible. HTTP 429 is deliberately excluded rather than ignoring Retry-After.
Exhausted HTTP errors retain their status in the technical details. Metadata
transport failures retain their underlying source chain (for example DNS, TLS
or timeout) in that same collapsed disclosure.

Generic connection errors are not retried: reqwest also reports TLS/certificate
failures through that category. This conservative change does not claim to fix
all network failures. JSON parsing, release/platform/version validation, signature
verification, artifact download, installation, consent and restart are outside
the retry loop and unchanged. No endpoint or certificate policy is changed.

Regressions use local HTTP fixtures to cover recovery, bounded exhaustion,
permanent statuses, 204/no update, malformed JSON, TLS failure and total timeout.
Frontend tests also cover duplicate check clicks and reopening the panel while
the native check is pending. Run the new native tests with:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml -p tauri-plugin-updater --lib updater::metadata::tests
```
