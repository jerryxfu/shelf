// The core's commands (src-tauri/src/commands.rs). Each one that changes the shelf answers with the shelf as it is now.

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
