# Releasing Mac-Word

Mac-Word is released from a Mac by hand; the repository has no hosted CI or release workflow.

1. Bump the version: `cargo xtask version set 1.0.1` (the one source of truth is
   `[workspace.package] version` in `Cargo.toml`; the app shows it in `--version` and About).
2. Run the checks locally: `cargo xtask ci`.
3. Build and package: `./package.sh`. It runs `./build.sh release` (universal `arm64` + `x86_64`,
   ad-hoc signed) and writes `release/Mac-Word-<version>.dmg` with an `Applications` link.
4. Install the DMG, launch it, and check the change.
5. Publish the DMG as a GitHub Release on `manikpruthi/mac-word`.

The app is ad-hoc signed and not notarized, so the first launch needs right-click › Open.

The scripts under `packaging/` for Windows, Linux and FreeBSD are inherited from upstream WordCraft
and are not part of Mac-Word releases.
