// The core's commands: the shelf's are in src-tauri/src/modules/shelf/commands.rs, the panel's in
// src-tauri/src/commands.rs. Each one that changes the shelf answers with the shelf as it is now. When the Shelf
// module is off, the shelf's commands reject with a message.

import {invoke} from "@tauri-apps/api/core";

export type ShelfItem = {
    id: number;
    path: string;
    name: string;
    /** In bytes. `null` for a folder. */
    size: number | null;
    isDir: boolean;
};

export const listItems = () => invoke<ShelfItem[]>("shelf_items");

export const addPaths = (paths: string[]) => invoke<ShelfItem[]>("shelf_add", {paths});

/** Takes an item off the shelf. The file stays where it is. */
export const removeItem = (id: number) => invoke<ShelfItem[]>("shelf_remove", {id});

export const hidePanel = () => invoke<void>("panel_hide");
