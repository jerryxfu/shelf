# Shelf

A desktop assistant for macOS and Windows, built from modules. It runs in the background, keeps files
where they belong, and gives you a place to hold files before you do something with them. A local AI
model is one of its modules, so nothing you sort ever leaves the machine.

> **Status:** the first prototype passes on macOS: a tray icon and a floating shelf panel that
> files can be dragged into and out of. It is small and light (a 3.3 MB app, 15 MB of memory when
> idle) and feels native, so Tauri stays. On Windows it builds but is untried. The core pieces
> that modules need are built too (26 Nov 2026): the list of modules, settings, and the action log
> with undo. The shelf is the first module, and Sift is next. See
> [docs/prototype-notes.md](docs/prototype-notes.md). The rest of this file records what has been
> decided and what is still open.

## Why

- Downloads, screenshots and the Desktop pile up, mostly with school files: lab handouts, notes,
  assignments.
- Local AI is weak next to frontier models at hard reasoning, but it is free, private and can run all
  day. "Which folder does this file belong in?" is a job a small local model does well.

## Goals

1. **Nothing piles up.** A school file ends up in the right course folder with almost no effort.
2. **Everything is visible and reversible.** Every action is logged with its reason and can be undone
   in one click.
3. **It adapts.** It reads the folders that exist today (they change every session) and learns from
   corrections.
4. **It is a platform.** A small core plus modules. New abilities are new modules.
5. **It stays local.** File contents go only to a model running on this machine.
6. **It runs on macOS and Windows.**
7. **It is lightweight.** Small installer, low memory when idle, nothing running that is not needed.
8. **Friends can use it.** It works with no AI and no setup. Each module can be switched on or off,
   so someone who only wants Expedite sees only Expedite.

## Modules

| Module       | What it does                                                                 | Status    |
|--------------|------------------------------------------------------------------------------|-----------|
| **Sift**     | Sorts new files (Downloads first, then Desktop screenshots) into folders      | First     |
| **Shelf**    | A drop spot that holds files until you act on them                           | Prototype |
| **Expedite** | Sends what is on the shelf as an [Expedite](https://jerryxf.net/expedite) drop | Planned   |
| **AI**       | Gives the other modules a local model to ask. The model can be swapped        | First     |
| **Pin**      | Keeps a chosen window on top of the others. Windows only                      | Planned   |

- **Modules are built into the app,** not loaded from outside. Each one lives in its own folder,
  with a short description: its name and the systems it supports. That keeps a module
  self-contained, so adding or removing one touches little else.
- **Each module can be switched on or off** in the settings. A module that is off starts nothing.
- **A module is hidden on a system it does not support.** macOS gives apps no supported way to keep
  another app's window on top, so Pin is hidden there.
- **AI is optional.** Modules that do not need it keep working when no model is running. Without a
  model, Sift still files with one click (see below).

### Sift

- **Trigger:** a new file appears in a watched folder. Half-finished downloads are skipped.
- **Clues:** file name, type, the first page or so of text, and the URL it was downloaded from.
- **Decision:** the model gets the clues and the current folder list, and returns a folder, an
  optional new name, a confidence and a one-line reason.
- **Action:** high confidence moves the file and shows a notification with Undo. Low confidence asks.
- **Starts in suggest-only mode.** Auto-move is switched on once it has earned trust.
- **New files show in the floating panel,** in a Sift view next to the shelf. Each one is a row with
  the current session's course folders as buttons. One click files it. A plain notification says
  that a file is waiting.
- **Without a model, Sift does not guess.** The row comes with no folder picked. If a folder's name
  is in the file's name, that folder comes first. This is built first.
- **With a model,** the same row arrives with one folder already picked, and the reason.

Rules Sift follows:

- **Never deletes, never overwrites.** It only moves and renames.
- **Renames only when it clearly helps,** and the original name always stays in the new name.
- **Touches only folders it was given.** It does not reorganize what is already sorted unless asked.
- **Sorts into the existing layout:** `Documents/Session N/<course>` in the school OneDrive. Course
  folders are read each time, never hard-coded.
- **Handles French and English** file names and contents.

### Shelf and Expedite

- The shelf is a place to drop files from anywhere and pick them up later.
- Expedite is the first action on shelved files: drop files, click send, get a code and a link.
- The [Expedite CLI](https://github.com/jerryxfu/expedite-cli) already does the upload and prints the
  drop's code on standard output, so the module can call it or use the same API.

### Ideas for later

- Clean up an existing folder on request, with a plan to review before anything moves.
- Find a file by describing it ("the physics lab handout from last week").
- Summarize a document when it arrives, or pull due dates from a syllabus.
- Create the course folders when a new session starts.
- A sort button for Sift's folder buttons: alphabetical or last modified.

## Architecture

```
trigger  ->  gather context  ->  ask the AI module (optional)  ->  propose or act  ->  log
```

- **Core:** module loading, settings, the action log with undo, notifications, the tray or menu bar
  icon. All of it is Rust, in one process.
- **Settings:** one JSON file, `settings.json`, in the system's folder for app data, owned by the
  core. A settings window comes after Sift works.
- **Action log:** kept by the core and shared by all modules. One text file, `actions.jsonl`, with
  one line per action. Undo moves the file back, and refuses if something is in the way.
- **Where those two files are:** `~/Library/Application Support/net.jerryxf.shelf/` on macOS,
  `%APPDATA%\net.jerryxf.shelf\` on Windows.
- **Interface:** one floating panel. The shelf and Sift each have a view in it.
- **AI module:** talks to an OpenAI-compatible local server (LM Studio at `http://localhost:1234`).
  Changing the model is a setting.
- **Core logic has no UI code in it,** so the interface can change without rewriting the modules.

### Framework: Tauri 2

- **One codebase for macOS and Windows:** a Rust core and a web interface.
- **No bundled browser.** It uses the system web view (WKWebView on macOS, WebView2 on Windows), so
  the installer stays small.
- **Idle cost is low.** Until a window is first opened only the Rust process runs: tray icon, file
  watching, modules (15 MB measured). Windows are created when shown and destroyed when hidden.
  After the first one, two small helper processes of the web view stay until the app quits (about
  39 MB in all with nothing open, 63 MB with the shelf open).
- **The two web views are different engines,** so the interface is tested on both systems.
- **Tray, menus, notifications and dialogs are the system's own.**

Considered and set aside: Kotlin Multiplatform with Compose (JVM memory use), native Swift plus a
separate Windows app (every interface feature built twice), Wails 3 (not yet stable).

**First prototype:** a tray icon and a floating shelf panel that files can be dragged into and out
of, on both systems. Dragging files out and a panel that does not steal focus both rely on plugins
(`tauri-plugin-drag` and, on macOS only, `tauri-nspanel`), so this is the riskiest part. Native
Swift was the fallback if it did not feel right.

- **Result on macOS (25 Nov 2026): it passes.** Checked by script and tried by hand: dragging in
  and out feels natural, clicking the panel leaves the app in use alone, and the panel looks at
  home. The Swift fallback is not needed.
- **Windows:** it builds on GitHub into an `.exe` and a 1.2 MB installer
  (`.github/workflows/windows.yml`, started by hand), but has never been run there.
- **To run it:** `pnpm install`, then `pnpm tauri dev`.
- **Measurements and limits found:** [docs/prototype-notes.md](docs/prototype-notes.md).
- **What to try on Windows:** [docs/windows-checklist.md](docs/windows-checklist.md).

What follows:

1. Test the prototype on Windows, with the checklist.
2. Build Sift in suggest-only mode: first without a model, then with the AI module.

Done since the prototype, on 26 Nov 2026: what modules need in the core. That is the list of
modules, settings, and the action log with undo. What was built and the choices still open are at
the end of [docs/prototype-notes.md](docs/prototype-notes.md).

### Model notes

Tested on a MacBook Pro M5 Pro with 64 GB, in LM Studio:

- A small model (about 4B, around 3 GB) should be enough for sorting. Candidates: Gemma 4 E4B,
  Qwen 3.5 4B. Both handle French.
- Cap the reply length and ask for structured (JSON) output. Qwen3.8 27B spent 20 minutes "thinking"
  on one question without answering until a token cap was added.
- **Turn thinking off for Sift.** It made a 3-second answer take a minute or more. LM Studio's chat
  API has no thinking switch for the Qwen models tested. What worked: the raw `/v1/completions`
  endpoint with a hand-written prompt that ends in an empty `<think></think>` block.
- Qwen3.6-35B-A3B runs at about 64 tokens per second, Qwen3.8 27B at about 9.5, with similar answers
  on the tasks tried. Qwen3.6-35B-A3B is the everyday model on the development Mac.

## Open decisions

None of these blocks the first version of Sift. The ones that did were settled on 25 Nov 2026 and
are written into the sections above.

- **Which small model** Sift uses by default.
- **Auto-move or ask first,** and the confidence needed for auto-move.
- **How Expedite is included:** bundle the CLI, or write the upload again in Rust.
- **The smaller choices made for the prototype and for the core pieces,** listed in
  [docs/prototype-notes.md](docs/prototype-notes.md).
