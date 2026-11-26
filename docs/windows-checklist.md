# Windows checklist

What to try on a Windows machine. The prototype builds for Windows, but nobody has run it there yet. About ten
minutes. Tick what works, and write a few words next to anything that does not or looks odd.

## Get the build

1. On the Windows machine, sign in to GitHub and open the repository's **Actions** tab, then **windows build**.
2. Open the latest run and download **shelf-windows** at the bottom, under Artifacts.
   - A download is kept for 14 days. The first one lasts until 10 Dec 2026.
   - If it is gone: **Run workflow** makes a new one in about 7 minutes.
3. Unzip it. There are two files: `shelf.exe` runs the app directly, `Shelf_0.1.0_x64-setup.exe` installs it.
4. Windows shows a blue "Windows protected your PC" box, because the app is not signed. Click **More info**, then
   **Run anyway**.

## Starting

- [ ] `shelf.exe` starts with no error box and no black console window.
- [ ] No window opens and nothing appears in the taskbar. Shelf is only an icon in the tray, bottom right. It may
      be hidden under the `^` arrow.

## Tray icon

- [ ] The icon is there. It is the placeholder app icon, not the shelf drawing used on macOS.
- [ ] Left click opens a menu with Show Shelf and Quit Shelf.
- [ ] Right click opens the same menu.
- [ ] Note which click feels right. On Windows, a left click usually opens the app and a right click the menu.

## The panel

- [ ] Show Shelf opens the panel just above the tray icon, whole on screen: not under the taskbar, not cut off.
      If the icon was under the `^` arrow, note where the panel opens.
- [ ] The panel is not in the taskbar and not in Alt+Tab.
- [ ] It stays above other windows.
- [ ] The menu now says Hide Shelf. After hiding, it says Show Shelf again.

## Focus

- [ ] Type in Notepad, then click inside the panel. Notepad stays the active window and keeps its typing cursor.

## Dragging files in

- [ ] Drag a file from Explorer onto the panel: a blue outline, then the file is listed with its name and size.
- [ ] A folder is listed as "Folder".
- [ ] The same file dropped twice is listed once.
- [ ] A name with an accent (`été.txt`) shows correctly.

## Dragging files out

- [ ] Drag a row into an Explorer folder: a copy arrives and the original stays where it was.
- [ ] Drag a row into another app: an email, a browser's upload box.
- [ ] It starts at once and feels natural. Note what the picture under the pointer looks like.
- [ ] Press, flick and release very fast a few times: nothing stays stuck to the pointer.

## Rows

- [ ] Rows and X buttons highlight under the pointer.
- [ ] The X on a row removes it from the shelf. The file itself is still on disk.

## Moving and hiding

- [ ] The panel can be dragged by its header, the bar that says Shelf. **This is the one most likely to fail on
      Windows:** note what happens.
- [ ] The X at the top right hides the panel. Show Shelf brings it back with the files still listed.

## Look

Expected on Windows for now: square corners and a plain background, with no blur behind. That part is macOS only.

- [ ] Text is sharp and nothing is cut off or misaligned.
- [ ] Same on a screen scaled to 125 % or 150 %.
- [ ] It is readable in light and in dark mode (Settings, Personalisation, Colours).
- [ ] Note whether the panel has a border or a shadow, and whether it flashes white when it opens.

## Installer

- [ ] The setup `.exe` installs Shelf, it appears in the Start menu, and it starts from there.
- [ ] It uninstalls from Settings, Apps.

## Numbers

From Task Manager. In the Processes tab, open the Shelf group to see its WebView2 processes and use the group's
total.

| State                         | Memory |
|-------------------------------|--------|
| Started, panel never opened   |        |
| Panel open with a few files   |        |
| Panel hidden again            |        |

For comparison, macOS: 15 MB, 63 MB and 39 MB.

## Quitting

- [ ] Quit Shelf removes the tray icon, and no `shelf.exe` is left in Task Manager.

## Only with a build made after 26 Nov 2026

The first download on GitHub is the prototype. It does not have the settings file yet. A build made after the core
pieces are pushed does, and then:

- [ ] After the first start, `settings.json` exists in `%APPDATA%\net.jerryxf.shelf\`.
- [ ] Quit, set `"shelf": false` in that file, start again: the menu only has Quit Shelf. Set it back to `true`
      after.
