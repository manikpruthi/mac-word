# Memory and start-up

Mac-Word targets an 8 GB MacBook. This page records what was measured, on which build, and which
limits keep memory bounded. Run the same steps to compare a future change.

## Methodology

Two scripts, both against a release build with an isolated profile:

- **`scripts/bench-foreground.py <Mac-Word.app> [document]`** — the numbers that matter. It
  starts the app with `open` so it comes to the front and draws, opens a 40-section,
  17,000-word document (or the one given), and prints the physical footprint and its main
  categories (`footprint -p PID`) at 100%, at page width, at 300% and at page width after moving
  to the end, each after a few idle seconds.
- **`scripts/bench-memory.py <binary> [--sample]`** — start-up time and the footprint right after
  launch, from a process started in the background.

Two traps to avoid when measuring:

- A window launched behind other windows isn't composited and its UI doesn't run, so zooming or
  scrolling it changes nothing and its GPU memory reads anywhere from 18 to 67 MB from run to
  run. Measure in front (`bench-foreground.py`).
- `ui.screenshot` keeps a window-sized capture texture (17 MB at the default window size) that
  ordinary use never allocates. `bench-memory.py` used to wait for the first frame with a
  screenshot, which inflated every number it reported; it now waits for `ui.inspect`.

macOS doesn't return freed large blocks to the system at once: `Malloc Large` in `footprint`
includes freed blocks the allocator keeps for reuse, which it gives back under memory pressure.

## Results

On November 27, 2026, `bench-foreground.py` ran the same generated document on three Mac-Word
release builds on an Apple-silicon Mac with macOS 27.2.1. The script checks each control response
and measures physical footprint after the app settles.

- At 100%, the original release (`ef7e961a`) used 143 MB. After the WordCraft sync (`092556ab`), it used 276 MB. With viewport tiles (`a8fb7615`), it used 215 MB.
- At page width, the three builds used 162 MB, 462 MB, and 229 MB, respectively.
- At 300%, they used 216 MB, 690 MB, and 237 MB.
- After returning to page width and scrolling to the end, they used 216 MB, 664 MB, and 254 MB.

At 300%, the large-malloc category fell from 389 MB to 22 MB and graphics memory fell from
127 MB to 39 MB after the tile fix. The newer build still has a higher footprint at 100% than
the original release. The installed arm64 package used ThinLTO for the app and CLI binaries.

Historical measurements on Apple silicon, macOS 27.2, release builds, 2026-11-03.
`bench-foreground.py` used the generated document:

| Step | DeepSeek-Word | Mac-Word |
|---|---:|---:|
| 100% zoom | 217 MB | **143 MB** |
| Page width | 304 MB | **162 MB** |
| 300% zoom | 645 MB | **216 MB** |
| Page width, scrolled to the end | 591 MB | **215 MB** |

The breakdown at 300%: DeepSeek-Word held 181 MB of GPU memory and 342 MB of large blocks
(whole-page rasters and their copies); Mac-Word holds 38 MB and 63 MB. Each step is measured
8 seconds after it, once the caret has stopped blinking. The window's two
drawables (33 MB at the default size) and the Rust heap (50–60 MB with this document) are the
same in both.

The installed app on its start screen (no document) is about 95 MB.

Upstream WordCraft 0.15.0 was not rebuilt for this comparison. DeepSeek-Word's own figures for it
(253 MB blank, 274 MB with the sample) were taken with the screenshot method above, so they read
about 17 MB high, like everything that method measured.

## Idle: the caret blink

Mac Excel, built on the same toolkit, idled at about 80 MB while Mac-Word idled at about 190 MB
with a document open. The heaps were alike (20–35 MB); the difference was GPU memory: 67 MB
against 3 MB. A build with switches for each suspect, each run in front on the built-in sample
and measured after 20 seconds:

| Variant | GPU memory | Footprint |
|---|---:|---:|
| As shipped before | 67 MB | 188 MB |
| No caret blink | 18 MB | 132 MB |
| Blink without its fade (2 redraws a second instead of about 20) | 66 MB | 187 MB |
| Blink stops after 5 s | 18 MB | 132 MB |
| No page rasters, blinking | 53 MB | 150 MB |
| No page rasters, no blink | 4.5 MB | 91 MB |
| Two frames in flight instead of one | 67 MB (window surfaces 49 MB, not 33 MB) | 206 MB |

Page rasters account for about 14 MB of GPU memory. About 50 MB is held only while the window
keeps redrawing, however rarely: every blink redraws and re-uploads the whole interface, and
Metal keeps the memory those uploads use until the redraws stop. Mac Excel's grid doesn't
animate, so it never holds it. Mac-Word now stops the blink, with the caret shown, 5 seconds after
the last key, click or caret move, as Word does on Windows' default settings; the memory is given
back a few seconds later. The built-in sample, idle: 120 MB (it was about 190 MB).

## What is bounded

| Resource | Upstream | Mac-Word | Why |
|---|---:|---:|---|
| Undo depth | 500 steps | 100 steps | Word keeps about 100; large documents no longer retain hundreds of full edit snapshots |
| Decoded image cache | 128 MB | 64 MB | Holds the photos of the pages on screen (five 12-megapixel photos), so a re-render after an edit doesn't decode them again; a picture larger than the whole cache is box-averaged to half of it |
| Decoded vector/metafile cache | 128 MB | 64 MB | Same reasoning |
| Frames in flight | 2 (three drawables) | 1 (two drawables) | Each drawable is a full-window IOSurface: 17 MB for the default window, 59 MB full screen on a 5K display |
| GPU on dual-GPU Macs | high performance (discrete) | low power (integrated) | The page is rasterised on the CPU; the GPU only composites a few textures. `WGPU_POWER_PREF` overrides it |
| wgpu indirect-draw validation | on (off on Intel) | off | The app issues no indirect draws; the validation shaders and buffers were start-up time and memory only |
| Media cache entries | 256 | 128 | Bounds bookkeeping as well as bytes |
| Editing canvas | Full-page textures for visible and nearby pages | Previews up to 512 px per side and visible 512 px tiles | Raster work and sharp texture memory follow the viewport at high zoom. Each tile uses the zero-copy `bytemuck::try_cast_vec` handoff |
| Caret blink | forever | stops after 5 s without input, caret shown | See [Idle: the caret blink](#idle-the-caret-blink): while the window keeps redrawing, Metal keeps about 50 MB of upload memory |
| Copies per rendered tile | 2 (raster, then the GPU image) | 1 (the raster's buffer becomes the image) | Zero-copy `try_cast_vec` avoids a buffer copy per uploaded page/tile image |
| Glyph outline cache | 50,000 | 10,000 | Ordinary documents use a few thousand; large CJK documents regenerate rather than keep every outline |
| Renderer workers | up to 8 | up to 4 | Keeps text rasterisation fast without eight long-lived worker stacks/pools |
| eframe system fonts | on | off | The app installs and falls back with its own font database; the duplicate scan held another copy |
| Default window | 1440×920 | 1280×820 | Smaller Metal surfaces and IOSurfaces for the same document area |

## Start-up

- **No update request.** The updater was removed with its dependency and URLs, so launch performs
  no network work.
- **Font catalog cache.** The first lookup of an installed font used to scan every font file on the
  Mac serially. Mac-Word writes the parsed catalog to `font-catalog.json` in the settings folder
  after the first scan and reuses it when every font folder and file still has the same timestamp.
  The first scan is spread across up to eight threads; later launches only stat the files.
- **Font picker catalog.** The macOS ribbon and mini toolbar use the document font information
  to display the current family. Hovering, clicking or focusing a font box starts the picker’s
  separate installed-font catalog in the background. Font dialogs can still load it directly.
- **Ribbon previews.** Styles, Style Sets and Table Styles cache each preview at its rendered
  size. A few visible previews no longer allocate a 2,048 × 2,048 shared RGBA texture.
- **Lazy proofing and documents.** The sample or requested document is built once, preferences are
  read once, and the window is shown before any optional work (language packs, update checks,
  Zotero) can run.
- **Smaller binary.** Fat LTO, one codegen unit and `strip = "symbols"` reduce the executable and
  the pages that must be loaded before the first frame.

## Notes

- macOS does not return freed heap pages to the OS immediately; compare the physical footprint
  after idle time, as the methodology does, not the peak during page layout.
- Metal heaps and IOSurfaces are shared with the window server. A smaller window and fewer cached
  page textures are the reliable ways to reduce them; the Rust heap is already small.
- Do not reduce the renderer to zero/one thread without measuring: page rasterisation is the
  responsiveness-critical path for large documents; four is the balance used here.
