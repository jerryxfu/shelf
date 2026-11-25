# Shelf

A desktop assistant for macOS and Windows, built from modules. It runs in the background, keeps files
where they belong, and gives you a place to hold files before you do something with them. A local AI
model is one of its modules, so nothing you sort ever leaves the machine.

> **Status:** planning. Nothing is built yet. This file records what has been decided and what is
> still open.

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

| Module       | What it does                                                                 | Status  |
|--------------|------------------------------------------------------------------------------|---------|
| **Sift**     | Sorts new files (Downloads first, then Desktop screenshots) into folders      | First   |
| **Shelf**    | A drop spot that holds files until you act on them                           | Planned |
| **Expedite** | Sends what is on the shelf as an [Expedite](https://jerryxf.net/expedite) drop | Planned |
| **AI**       | Gives the other modules a local model to ask. The model can be swapped        | First   |
| **Pin**      | Keeps a chosen window on top of the others. Windows only                      | Planned |

- **AI is optional.** Modules that do not need it keep working when no model is running. Sift falls
  back to simple rules (file type, the site a file came from) without a model.
- **A module declares which systems it supports.** macOS gives apps no supported way to keep another
  app's window on top, so Pin is hidden there.

### Sift

- **Trigger:** a new file appears in a watched folder. Half-finished downloads are skipped.
- **Clues:** file name, type, the first page or so of text, and the URL it was downloaded from.
- **Decision:** the model gets the clues and the current folder list, and returns a folder, an
  optional new name, a confidence and a one-line reason.
- **Action:** high confidence moves the file and shows a notification with Undo. Low confidence asks.
- **Starts in suggest-only mode.** Auto-move is switched on once it has earned trust.

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

## Architecture

```
trigger  ->  gather context  ->  ask the AI module (optional)  ->  propose or act  ->  log
```

- **Core:** module loading, settings, the action log with undo, notifications, the tray or menu bar
  icon.
- **AI module:** talks to an OpenAI-compatible local server (LM Studio at `http://localhost:1234`).
  Changing the model is a setting.
- **Core logic has no UI code in it,** so the interface can change without rewriting the modules.

### Framework: Tauri 2

- **One codebase for macOS and Windows:** a Rust core and a web interface.
- **No bundled browser.** It uses the system web view (WKWebView on macOS, WebView2 on Windows), so
  the installer stays small.
- **Idle cost is low.** With no window open only the Rust process runs: tray icon, file watching,
  modules. Windows are created when shown and destroyed when hidden.
- **The two web views are different engines,** so the interface is tested on both systems.
- **Tray, menus, notifications and dialogs are the system's own.**

Considered and set aside: Kotlin Multiplatform with Compose (JVM memory use), native Swift plus a
separate Windows app (every interface feature built twice), Wails 3 (not yet stable).

**First thing to prototype:** a tray icon and a floating shelf panel that files can be dragged into
and out of, on both systems. Dragging files out and a panel that does not steal focus both rely on
plugins, so this is the riskiest part. Native Swift is the fallback if it does not feel right.

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

- **Which small model** Sift uses by default.
- **Where the core logic lives:** all in Rust, or partly in a bundled Go helper.
- **Auto-move or ask first,** and the confidence needed for auto-move.
- **What the shelf looks like:** a floating panel, a menu bar popover, or a window.
- **How Expedite is included:** bundle the CLI, or write the upload again in the app's language.
