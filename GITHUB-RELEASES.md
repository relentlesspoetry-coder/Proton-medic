# GitHub and signed updates

Repository: https://github.com/relentlesspoetry-coder/proton-medic

The v0.6.6 app has a Settings → App updates panel. It checks the stable release manifest only when clicked, compares versions, shows release notes, and requires approval before installing. The native Tauri updater verifies the package's signature and signed version. No source checkout, shell download script, embedded GitHub token, or automatic restart is used.

## One-time repository setup

1. Create the public repository `relentlesspoetry-coder/proton-medic`. When using the ChatGPT GitHub connector, initialize it with a README and grant the connector access to this repository.
2. Push the prepared source. Do not upload the separate private signing-key backup, build caches, personal logs, or old source backup archives.
3. In GitHub Settings → Secrets and variables → Actions, add `TAURI_SIGNING_PRIVATE_KEY`. Paste the complete contents of the separately supplied `proton-medic.key` file. This key has an empty password; `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` may be omitted. Never place the key in a repository file, issue, or release.
4. Keep a secure offline copy of the private key. The application embeds the matching public key; changing the signing key without a migration prevents existing installations from accepting future releases.
5. After the source and secret are in place, either push a matching stable tag or open Actions → Signed Linux release → Run workflow on `main` and enter `v0.6.6`. The manual path validates the version, builds and verifies the signed installer, then creates the tag at the workflow's exact source commit and publishes. It fails if that tag already exists; do not move an existing release tag. Both paths run `.github/workflows/release.yml`.

On 2026-10-28, GitHub-hosted validation and the signed release workflow passed, and v0.6.6 was published with the DEB, signature, SHA256SUMS, and latest.json. The source before release setup is preserved on `backup/v0.6.6-before-release-setup` at `fffdd4526dda879f92e8febbc4e2fee1537a9184`. The v0.6.6 tag points to `1eb4fefb65d201f6ac8b16adaca41267724b5457`; only the release workflow changed from the backed-up source. Live in-app installation and desktop GUI/gameplay remain unverified. See `RELEASE-v0.6.6.md` for verification details.

## Future releases

Update `package.json`, the root application entries in `package-lock.json` and `src-tauri/Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and visible/native version labels. Do not globally replace dependency versions in lockfiles.

Commit the change and push a matching stable `vX.Y.Z` tag. The release workflow:

- Verifies that the tag and application versions agree.
- Runs frontend, native, disposable fixture, build-gate, and release-manifest tests.
- Builds the native amd64 DEB on Ubuntu 24.04.
- Signs the exact installer with its application version.
- Verifies its signature with the application's public key and proves a modified package is rejected.
- Publishes the DEB, signature, SHA256SUMS, and `latest.json` only after all checks pass.

The signing key is exposed only to the signing step, not pull-request validation. The update manifest advertises only `linux-x86_64-deb`; it does not send a DEB to an AppImage, ARM, or RPM installation. Additional package formats need their own tested build and manifest entries. Package uploads are version-tagged; the `releases/latest/download/latest.json` URL is the stable discovery endpoint.

## Local signed package verification

After running `bash scripts/build-zorin.sh --build`, copy the resulting DEB to `release/proton-medic_VERSION_amd64.deb`. With the private key outside the repository:

```bash
npm run tauri -- signer sign --private-key-path /secure/path/proton-medic.key --password '' --app-version VERSION release/proton-medic_VERSION_amd64.deb
cargo run --locked --manifest-path scripts/verify-signature/Cargo.toml -- src-tauri/tauri.conf.json release/proton-medic_VERSION_amd64.deb VERSION
python3 scripts/release_manifest.py --tag vVERSION --package release/proton-medic_VERSION_amd64.deb --output release/latest.json
```

A DEB update uses the system's privileged package installation flow. Cancellation, signature errors, missing releases, unsupported targets, and network failures are reported as failures, never as a successful update. Finish game installations and repairs before updating. Close and reopen Proton Medic yourself after installation.
