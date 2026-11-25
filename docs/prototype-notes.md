# Prototype notes

The riskiest part of Shelf, tried with Tauri 2: a tray icon and a floating shelf panel that files can be dragged into
and out of. Built and measured on 25 Nov 2026, on a MacBook Pro M5 Pro with macOS 27.2.

> **Verdict: Tauri 2 passes on macOS.** It is small and light, and both risky parts work through plugins. Claude
> checked what a script can drive, then Jerry tried it [by hand](#tried-by-hand-by-jerry-25-nov-2026): dragging in
> and out feels natural, clicking the panel leaves the app in use alone, and the panel does not look out of place.
> **Windows:** the code compiles for it (checked from the Mac) but has never run there. That is the next thing to
> check.

## What is in the prototype

- **Tray icon** (menu bar icon on macOS) with a menu: Show Shelf or Hide Shelf, and Quit Shelf.
- **No Dock icon** on macOS.
- **Shelf panel:** 300 by 380, floats above other windows, opens under the tray icon, can be moved by its header.
- **Drag in:** files and folders dropped on the panel are listed with name and size.
- **Remove:** the X on a row takes it off the shelf. The file on disk is never touched.
- **Drag out:** a row can be dragged to Finder or another app.
- **Low idle cost:** hiding the panel destroys its window and web view. Showing it makes a new one.

Not in it, on purpose: AI, Sift, Expedite, settings, the module system.

## How to run it

```bash
pnpm install
```

```bash
pnpm tauri dev
```

The first run compiles about 230 Rust crates: 25 seconds on this Mac, with every core busy. After that, a change to
Shelf's own code rebuilds in 2 to 4 seconds. A packaged app:

```bash
pnpm tauri build --bundles app
```

That takes about 50 seconds and writes `src-tauri/target/release/bundle/macos/Shelf.app`. The build folder
(`src-tauri/target`) has grown to 5.5 GB; `cargo clean` in `src-tauri` empties it.

## Layout

| Path                         | What it holds                                                          |
|------------------------------|------------------------------------------------------------------------|
| `src/`                       | The interface: React, TypeScript, SCSS, built by Vite                  |
| `src/shelf/`                 | The panel, the calls to the core, drag out                             |
| `src-tauri/src/shelf.rs`     | The shelf itself. Plain Rust: no Tauri, no window code. Has unit tests |
| `src-tauri/src/commands.rs`  | What the interface may ask the core to do                              |
| `src-tauri/src/panel.rs`     | The panel's window: create, place, destroy. One part per system        |
| `src-tauri/src/tray.rs`      | The tray icon and its menu                                             |
| `src-tauri/src/dev.rs`       | A test aid, only built with `--features dev-control` (see below)       |

The shelf's list lives in the Rust core, not in the page. That is what lets the web view be destroyed while the shelf
keeps its files, and it keeps core logic free of interface code. About 1000 lines in all.

## Measurements

Release build, on this Mac. Memory is the "Memory" column of Activity Monitor (physical footprint), added up over
Shelf and the WebKit helper processes that work for it.

| State                                  | Memory    | Processes                                           |
|----------------------------------------|-----------|-----------------------------------------------------|
| Started, panel never opened            | **15 MB** | Shelf alone                                         |
| Panel open, 3 files listed             | **63 MB** | Shelf 25, web content 20, GPU 13, networking 5      |
| Panel hidden again                     | **39 MB** | Shelf 24, GPU 10, networking 5                      |

- **Processor when idle:** 0 % in all three states.
- **Reopening the panel:** about 65 ms from Show to the list being painted (64, 72 and 63 ms over three tries). So
  destroying the web view on hide costs nothing you can see.
- **App size:** `Shelf.app` is **3.3 MB**, and 1.6 MB zipped. Apple Silicon only; a build for Intel Macs too would be
  about twice that.
- **Interface bundle:** 247 KB of JavaScript (76 KB gzipped), most of it React.

One thing differs from the plan. The README said that with no window open only the Rust process runs. That is true
until the panel is first opened. After that, two WebKit helpers (GPU and networking, 15 MB together) stay until Shelf
quits. They were still there more than a minute after hiding. Destroying the web view still saves 24 MB each time.

## What works

Checked by Claude on this Mac, by driving the app from a script (see [how](#how-it-was-tested-without-a-mouse)).

| What                                                                                   | Result |
|----------------------------------------------------------------------------------------|--------|
| `pnpm tauri dev` builds and starts the app                                             | Yes    |
| The tray icon is in the menu bar                                                       | Yes: the system reports it visible, with a place just right of the notch |
| No Dock icon                                                                           | Yes, in dev and in the packaged app |
| Quit                                                                                   | Yes: gone in under 200 ms, helpers too |
| The panel opens under the tray icon, 6 points below the menu bar                       | Yes, after a workaround (below) |
| Showing the panel does not bring Shelf to the front                                    | Yes: Firefox stayed the front app through every show and hide |
| The panel is a real non-activating macOS panel                                         | Yes: `NSPanel`, non-activating style, floating level, not the key window |
| The page renders in the system web view, in dev and packaged (with a content security policy) | Yes |
| Files held over the panel light it up; leaving clears it                               | Yes, with simulated events, and by hand from Finder |
| Dropped files are listed with name and size                                            | Yes, with simulated events, and by hand from Finder. A French name with an accent, a folder and a missing file all behave |
| Dropping the same files twice                                                          | No duplicates |
| Remove takes the row away and leaves the file on disk                                  | Yes, the file was read back intact |
| Hide destroys the window and its web content process                                   | Yes |
| The list is still there when the panel is shown again                                  | Yes |
| Windows or processes piling up over many show and hide rounds                          | None |
| Dragging a row out to Finder and to another app                                        | Yes, by hand: it starts at once and feels natural |
| The menu says Show Shelf or Hide Shelf to match the panel                              | Yes, after a fix (below) |
| Rust unit tests, linter (`cargo clippy`), formatter, TypeScript check                  | 4 of 4 pass, no warnings |

"Simulated events" means the drop was injected where Tauri hands it to the page. The step before that, macOS giving
the drop to Tauri, is covered by the drags done by hand.

## What felt off or needed a workaround

1. **Placing the panel under the tray icon.** Tauri's `monitor_from_point` compares in points on macOS while the
   tray icon's position comes in pixels. On a Retina screen it found no screen, and the panel opened in the middle.
   Fixed in `panel.rs` by looking through the screens ourselves. A Tauri quirk worth reporting upstream.
2. **WebKit helpers stay after the web view is destroyed** (see Measurements). Nothing in Tauri's API ends them.
3. **Drag out offers copy or move, not both.** Finder offers both and lets the place you drop on choose. The plugin
   makes you pick one per drag. The prototype uses copy: the file stays where it was and the destination gets a copy.
   With move, apps that only accept copies (a browser's upload box, a chat app) would refuse the drop.
4. **The picture under the pointer while dragging must be a PNG you supply.** The prototype draws a plain sheet of
   paper. The file's real icon would need a little native code in the core.
5. **A drag that starts with no mouse button held gets stuck.** Seen by accident: a test script sent fake pointer
   events to a row, the plugin started a real drag, and with no button down macOS kept it waiting. Shelf then did
   not quit normally and had to be killed. A person cannot do exactly that, but a very quick press, flick and
   release might release the button before the drag begins. Jerry tried fast flicks by hand and nothing got stuck.
   If it ever shows up, the fix is a check in our own drag code (next point).
6. **If 3 or 4 matter,** the `drag` crate under the plugin is about 250 lines for macOS and 400 for Windows.
   Replacing it with our own code is realistic, and would cover 5 as well.
7. An invisible 500 by 500 window belonging to Shelf appears after the first Show and stays. It does not multiply.
   Not looked into.
8. pnpm 12 refuses to install while a package's build script is unreviewed. `pnpm-workspace.yaml` marks
   `@parcel/watcher` (an optional part of Sass, not needed here) as not allowed.
9. **The menu kept saying Hide Shelf after the panel was hidden.** Found by Jerry by hand, missed by the scripted
   checks, which never read the menu's text. Cause: Tauri keeps a destroyed window in its list for a moment, and
   the menu's text was decided from that list right after hiding. Fixed: `panel.rs` now keeps its own record of
   whether the shelf is open. The test aid's `state` command prints the menu's text, and the fix was checked with it.
10. A leftover of the same cause: Hide followed by Show in the very same instant fails to reopen, because the old
    window is not gone yet. The panel stays closed, the menu stays right, and the next Show works. Only a script can
    be that fast. Not fixed.

## Plugins

Versions are the current ones on 25 Nov 2026, read from crates.io and npm, not from memory.

| Piece               | Version | For                                      | How mature it looks                                                                 |
|---------------------|---------|------------------------------------------|-------------------------------------------------------------------------------------|
| `tauri`             | 2.12.7  | Everything                               | Stable, 20 million downloads in 90 days. Tauri 3 is in alpha: stay on 2 for now     |
| `tauri-plugin-drag` | 2.1.1   | Dragging files out                       | By CrabNebula, a company run by Tauri maintainers. Small. Last release May 2026, repository still active. 100 000 downloads in 90 days |
| `tauri-nspanel`     | 2.1.1   | A panel that does not take focus (macOS) | One maintainer. On crates.io since Sept 2026, 60 000 downloads. Used by Cap, Handy, screenpipe, EcoPaste |

Dragging files **in** needs no plugin: Tauri gives the dropped files' paths itself.

### Is `tauri-nspanel` needed?

Yes on macOS. Confirmed by hand: with it, clicking the panel leaves the other app its menu bar and typing cursor.

- A normal Tauri window on macOS is an `NSWindow`. Clicking it activates the app: Shelf would come to the front and
  the app in use would lose its menu bar and its focus.
- Tauri has `focusable(false)`. On macOS it only stops the window from becoming the key window. The click still
  activates the app.
- Only an `NSPanel` with the "non-activating" style avoids that. That is what the plugin gives, and the running
  panel does report that style.
- **On Windows the plugin is not used.** Tauri's `focusable(false)` sets `WS_EX_NOACTIVATE` there, which is the
  Windows way to do the same thing.

What to know about it:

- It works by swapping the window's class to a panel class while the app runs. That is a known trick, not an official
  Tauri feature, and it could break with a Tauri or macOS update.
- It calls one private macOS method (`_setPreventsActivation:`). Fine outside the Mac App Store, a risk inside it.

## How it was tested without a mouse

This session cannot move the mouse or take screenshots on the Mac: macOS did not give it those permissions. So:

- `--features dev-control` adds a few commands read from standard input: `show`, `hide`, `toggle`, `state`, `quit`,
  and `eval <script>`, which runs a script in the panel's page. It is off in a normal build, and the packaged app
  measured for size does not contain it.
- Memory was read with `footprint`, windows with the system's window list, the front app with `lsappinfo`.
- `pnpm tauri dev` does not pass standard input on, so the scripted checks ran on the binary it builds, started
  directly. `pnpm tauri dev` itself was checked to build, start, and show no Dock icon.

What that does **not** cover is what only a hand on the trackpad shows. Jerry did that part.

### Tried by hand by Jerry, 25 Nov 2026

With `pnpm tauri dev`, on the same Mac.

| #  | What                                                                                  | Result |
|----|---------------------------------------------------------------------------------------|--------|
| 1  | The icon is in the menu bar; Show Shelf opens the panel under it                      | Yes    |
| 2  | Clicking the panel leaves the other app its menu bar and typing cursor                | Yes    |
| 3  | A file dragged from Finder gives a blue outline, then is listed                       | Yes    |
| 4  | Dragging a row out to Finder and to another app starts at once and feels natural      | Yes    |
| 5  | Fast press, flick and release: a picture stuck to the pointer?                        | No, it is fine |
| 6  | Rows and X buttons highlight under the pointer                                        | Yes    |
| 7  | The panel can be dragged by its header                                                | Yes    |
| 8  | The X on a row removes it and the file stays                                          | Yes. Clicking the panel's own X is what showed the menu bug (point 9 above, fixed) |
| 9  | Hide, then show: the files are still listed                                           | Yes    |
| 10 | The look: corners, shadow, blurred background, light and dark                         | Yes    |

In Jerry's words: the app looks really clean and extremely natural, it does not feel out of place, and hovers and
colours all work.

Points 2, 4 and 5 were the ones that decided between Tauri and the Swift fallback. All three pass.

## Choices made to get a prototype, all open

- The panel opens under the tray icon every time. Its position is not remembered.
- A click on the tray icon opens the menu. It could toggle the shelf directly.
- Drag out copies. The item stays on the shelf afterwards.
- An item whose file no longer exists is dropped from the shelf the next time the panel opens.
- Folders show "Folder", not a size.
- The shelf is forgotten when Shelf quits.
- The interface is in English only.
- The icons (app, tray, drag picture) are placeholders.
- No ESLint yet. The TypeScript settings are the ones from `jerrydev`.
- The app is not signed with a developer certificate or notarized. Friends would need that to open it without a
  warning, as with the Expedite CLI.

## To test on Windows

None of this has run on Windows. What is known so far:

- **The code compiles for Windows.** Checked from the Mac with `cargo check --target x86_64-pc-windows-msvc`: no
  errors, and no linter warnings. That covers the Windows part of `panel.rs`, written from the documentation, and
  every Tauri and plugin call it makes.
- **A check is not a build.** Nothing was linked into an `.exe`: that needs Microsoft's linker and libraries, so a
  Windows machine or a Windows build on GitHub Actions. The step that puts the icon in the `.exe` was skipped too
  (this Mac has no Windows resource compiler, so a stand-in that writes an empty file took its place, set with
  `RC_x86_64_pc_windows_msvc`).

**Getting a Windows build without installing Rust on Windows:** `.github/workflows/windows.yml` builds it on
GitHub. It has not run yet: it only exists on GitHub once it is pushed.

1. Push the repository, with the workflow file.
2. On GitHub: Actions, "windows build", Run workflow. The first run compiles everything and may take 10 to 15
   minutes; later runs reuse the compiled crates.
3. Download "shelf-windows" from the run's page. It holds `shelf.exe` (the app alone) and a setup `.exe` (the
   installer).
4. On Windows, run either. They are not signed, so Windows shows a blue "Windows protected your PC" box: More
   info, then Run anyway.

Still to do on a real Windows machine:

- It builds into an `.exe` and starts.
- The tray icon and its menu. Left click and right click.
- The panel opens above the taskbar, not under it, and does not appear in the taskbar.
- Clicking the panel does not take focus from the app in use (`WS_EX_NOACTIVATE`).
- Drag in from Explorer, drag out to Explorer and to another app.
- Dragging the panel by its header, which may not work on a window that cannot take focus.
- The look: there is no blur behind the panel on Windows, and its corners are square.
- WebView2 is a different engine from macOS's: check the layout.
- Memory when idle and installer size. WebView2 has its own helper processes, so the numbers will differ.

## Recommendation

**Keep Tauri 2. The Swift fallback is not needed on macOS.**

- The lightweight goal is met with room to spare: a 3.3 MB app, 15 MB when idle, 63 MB with the panel open.
- The two risky parts both work, by script and by hand: the panel is a real non-activating panel that leaves the app
  in use alone, and files drag in and out naturally.
- The rough edges left are in the drag plugin (copy or move, the picture under the pointer), and they have a known
  way out: write those 250 lines ourselves. That is far less work than a native Swift app plus a separate Windows
  app.
- **Windows is the remaining unknown.** The code compiles for it, but nobody has run it. It needs its own pass
  on a real machine before building modules on top: the list is above.
