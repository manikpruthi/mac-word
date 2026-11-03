# Mac-Word fork notes

Mac-Word is a fork of [storytold/wordcraft](https://github.com/storytold/wordcraft), pinned at
upstream commit `ae5c65b9dd11faf359ce039a2a487e57e3d824a2` (fetched 2026-11-03), released here as
version **1.0.0**. The upstream remote is `upstream`; no changes are pushed back to it.

## How Mac-Word was assembled

Two AI-made forks of WordCraft at the same upstream commit were compared on 2026-11-03:

| Fork | Score | Strengths | Weaknesses |
|---|---:|---|---|
| `DeepSeek-Word` | 7/10 | Measured memory work with a benchmark script; font-catalog cache; Arrange All, Side by Side, Synchronous Scrolling, Insert › Screenshot; full README and fork docs; built and ran | Synchronous Scrolling fought itself with documents of different lengths; About dropped the dictionary licence texts; a hard-coded author name in every saved document; a 24 MB picture cache that re-decoded photos on every re-render |
| `Alpha-Word` | 5/10 | Thorough rebrand across all packaging; robust tests on machines without the language data or with a French dictionary; `xattr -cr` before signing; Arrange All | No measurements (mimalloc added on an unmeasured claim); env vars renamed without the old names; README's icon is an `.icns` in an `<img>` (doesn't render on GitHub); one screenshot; not yet built |

`mac-word` started from `DeepSeek-Word`, took `Alpha-Word`'s test fixes and signing fix, then
fixed the bugs above and cut GPU memory (see [memory.md](memory.md)).

## Identity changes

| Area | Upstream | Mac-Word |
|---|---|---|
| Product name | WordCraft | Mac-Word |
| Version | 0.15.0 | 1.0.0 |
| Bundle id | `ai.storyteller.wordcraft` | `com.manikpruthi.mac-word` |
| Executable | `WordCraft` (`wordcraft`) | `Mac-Word` (`wordcraft` package, packaged binary renamed) |
| CLI | `wordcraft-cli` | `mac-word-cli` (packaged rename; package name unchanged) |
| Settings | `~/Library/Application Support/WordCraft` | `~/Library/Application Support/Mac-Word` |
| Logs | `logs/wordcraft.log` | `logs/mac-word.log` |
| About | contributor and model tabs, upstream links | “Mac-Word”, “Version 1.0.0”, “From Manik Pruthi”; no tabs; the Contributors, Models and Language data pages stay, opened on their own (the dictionaries' licences require their text) |
| Default comment/track-changes author | WordCraft User | The macOS account's full name (`id -F`) until one is set in File › Options; `Mac-Word User` in the CLI and MCP server |
| Icons | WordCraft artwork | Original Mac-Word icon (`packaging/icons/mac-word-icon.swift`) |
| Update checks | craft-update at startup and Help menu | Removed entirely (dependency, UI, command and URLs) |
| Third-party UI | Discord button, ArtCraft links | Removed |
| Brand assets | `docs/brand/` | Removed (the upstream licence requires forks to remove them) |

Internal Rust crate and module names remain `wordcraft-*`. They are not user-visible, they keep
upstream diffs readable, and renaming them would make future upstream merges much harder. The
user-facing surface — process, bundle, window title, menus, About, settings, logs, PDF metadata,
MCP server name and CLI messages — says Mac-Word.

## Update and telemetry removal

The fork removes:

- the `craft-update` workspace dependency and its uses in the desktop app and UI;
- `crates/ui-egui/src/updates_ui.rs` (startup check, manual check, update prompt, restart helper);
- the `ui.checkForUpdates` / `help.checkForUpdates` commands and the Help-tab button;
- the Windows `--finish-update` self-update helper in `apps/wordcraft/src/main.rs`;
- the `UPDATE_APP` descriptor and every upstream update URL.

There is no update check, no background version request, no telemetry and no account system.

Environment variables support the Mac-Word names first and the upstream names for compatibility:
`MACWORD_PROFILE_DIR` / `WORDCRAFT_PROFILE_DIR`, `MACWORD_CONTROL_PORT` /
`WORDCRAFT_CONTROL_PORT`, `MACWORD_NO_PREFS` / `WORDCRAFT_NO_PREFS`, `MACWORD_CONTROL_KEY` /
`WORDCRAFT_CONTROL_KEY`, and `MACWORD_DICTIONARIES` / `WORDCRAFT_DICTIONARIES`. The default custom
dictionary folder is `~/Library/Application Support/Mac-Word/dictionaries`.

## Feature additions

- **View › Window › Arrange All**: tiles every open native window over the screen.
- **View › Window › View Side by Side**: places the active window and the previously focused one
  across the screen and enables synchronous scrolling, as Word does.
- **View › Window › Synchronous Scrolling**: the window being scrolled (under the pointer, or the
  focused one) moves its side-by-side partner by the same distance, as Word does
  (`WordApp::sync_scroll_to_pair`). With a single window, Side by Side and Synchronous Scrolling
  report that a second window is needed and stay off.
- **Insert › Screenshot**: interactive selection through macOS's `screencapture`, inserted as a
  picture. The engine command only asks for it; the desktop front end runs the capture on its own
  thread (the window keeps drawing) and inserts the result with `insert.picture`. Screen
  Recording permission is requested by the system on first use.
- **Word file types**: `.docm`, `.dotm`, `.dot`, `.mht`/`.mhtml` and Word XML are registered in
  `Info.plist` next to `.docx`, `.dotx`, `.doc`, `.odt`, `.rtf`, Markdown, HTML and text, with
  imported declarations of their public type identifiers, so they resolve on a Mac without Word.
  Which types Word registers was checked with REA in Word's `Info.plist`, an exception to the
  clean-room rule in `CLAUDE.md` (bundle file listings only) made at the owner's request: only
  the identifiers and extensions, which interoperability needs, were used; the descriptions are
  Mac-Word's own wording. Mac-Word stays an `Alternate` handler: it never becomes
  the default app without a click in File › Options › General.

The parity checklist is regenerated with `cargo xtask parity`; the four additions above raise the
live count from 505 to 509 of 520 catalogued features.

## Memory and start-up changes

See [memory.md](memory.md) for measurements and reproduction steps. In short:

- undo depth 500 → 100;
- image and vector decode caches 128 MB each → 64 MB each, 128 entries;
- one frame in flight (two window drawables instead of three) and the integrated GPU on dual-GPU Macs;
- wgpu's indirect-draw validation off on every Mac (the app issues no indirect draws);
- pages drawn from 256-pixel tiles, keeping only the tiles on screen and within half a viewport
  height of it, instead of whole pages within three viewport heights; one copy fewer per tile;
- the caret stops blinking, shown, 5 seconds after the last key, click or caret move: a window
  that keeps redrawing keeps about 50 MB of Metal memory in use;
- glyph outline cache 50,000 → 10,000;
- renderer worker threads up to 8 → up to 4;
- eframe's duplicate system-font fallback disabled;
- default window 1440×920 → 1280×820;
- installed-font catalog cached in `font-catalog.json` in the settings folder, validated against
  every font folder and file timestamp;
- first font scan parallelised across up to 8 threads;
- release profile: fat LTO, one codegen unit, symbols stripped.

## Building and testing

```sh
# native, with full language data
CRAFT_I18N_DIR="$PWD/../craft-i18n" cargo build --release -p wordcraft -p wordcraft-cli

# universal app and DMG
./build.sh release
./package.sh

# checks
CRAFT_I18N_DIR="$PWD/../craft-i18n" cargo xtask ci
```

`build.sh` uses `$HOME/Library/Caches/MacWordBuild` by default; `MACWORD_BUILD_DIR` overrides it.
`package.sh` writes `release/Mac-Word-1.0.0.dmg`.

## Pulling upstream changes

```sh
git fetch upstream
git log --oneline HEAD..upstream/main
git diff upstream/main --stat
```

Keep internal crate names and the command ids intact when merging: the engine, tests and parity
catalog assume them. Re-apply the update-removal patch if upstream adds new updater code, and keep
the Mac-Word strings and packaging files. Run `cargo xtask parity` after merging catalog changes.

## Known differences from Word

Mac-Word is not a complete Word implementation; the ranked gaps live in [gaps.md](gaps.md) and the
generated [parity checklist](parity-checklist.md). The most visible are external services and
inputs (Online Pictures/Video, 3D Models, Add-ins, Dictate, Translate, Ink to Math) and full
Block Authors. Charts and SmartArt made in Word are displayed read-only; macros are preserved but
not executed.
