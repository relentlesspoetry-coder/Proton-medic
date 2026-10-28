# Signed releases and app updates

Repository: https://github.com/relentlesspoetry-coder/Proton-medic

Proton Medic checks GitHub for a newer stable version when the update button is clicked. It displays the available version and release notes, asks for installation approval, and verifies the package signature and signed version before installing.

In v0.6.6, updates are in Settings. In v0.6.7, the update button is at the top right on every page, with the installed version beneath it.

## Signing key

Store the existing maintainer key in the repository's Actions secret named `TAURI_SIGNING_PRIVATE_KEY`. Use the complete contents of `proton-medic.key`. The existing key has an empty password, so `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` can be omitted.

Keep a secure offline backup. Never commit the private key or attach it to an issue or release. Existing installations trust the matching embedded public key; a key change requires a migration.

## Prepare a version

Update the application version in:

- `package.json`
- The root application entries in `package-lock.json` and `src-tauri/Cargo.lock`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

The header label reads the package version, and the native startup message reads the Cargo package version. Leave dependency versions unchanged unless updating those dependencies.

Add `RELEASE-vX.Y.Z.md` with the changes, new features, relevant usage instructions, and package requirements. Keep release notes focused on the product.

Back up the previous source before making release changes.

## Publish

Commit the source, then use either method:

1. Push a stable tag matching the application version, such as `v0.6.7`.
2. Open **Actions → Signed Linux release → Run workflow**, select `main`, and enter the matching tag.

The manual workflow creates the tag at its source commit after verification. It stops if that tag already exists. Keep published release tags unchanged.

The workflow:

- Checks that the tag matches the application version.
- Runs frontend, Rust, fixture, build-gate, and manifest tests.
- Builds the amd64 DEB on Ubuntu 24.04.
- Signs the installer with its version.
- Verifies the signature and rejects an altered copy.
- Publishes the DEB, signature, `SHA256SUMS`, and `latest.json`.

Signing credentials are available only to the signing step. The manifest currently supports `linux-x86_64-deb`.

## Verify a local package

After building, copy the DEB to `release/proton-medic_VERSION_amd64.deb`. Keep the private key outside the repository.

```bash
npm run tauri -- signer sign --private-key-path /secure/path/proton-medic.key --password '' --app-version VERSION release/proton-medic_VERSION_amd64.deb
cargo run --locked --manifest-path scripts/verify-signature/Cargo.toml -- src-tauri/tauri.conf.json release/proton-medic_VERSION_amd64.deb VERSION
python3 scripts/release_manifest.py --tag vVERSION --package release/proton-medic_VERSION_amd64.deb --output release/latest.json
```

## Install an update

Finish active game installations and repairs, check for updates, review the release notes, and approve installation. The system package manager may request administrator approval. Close and reopen Proton Medic after installation.

The stable manifest URL is:

https://github.com/relentlesspoetry-coder/proton-medic/releases/latest/download/latest.json

Requirements: Ubuntu 24.04-compatible amd64 Linux, glibc 2.39 or newer, GTK 3, and WebKitGTK 4.1.
