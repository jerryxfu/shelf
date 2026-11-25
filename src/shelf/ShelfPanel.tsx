import {useCallback, useEffect, useState} from "react";
import {getCurrentWebview} from "@tauri-apps/api/webview";
import {File, Folder, Inbox, X} from "lucide-react";

import "./ShelfPanel.scss";
import {addPaths, hidePanel, listItems, removeItem, type ShelfItem} from "./api.ts";
import {isDraggingOut, useDragOut} from "./dragOut.ts";
import {formatSize} from "./format.ts";

export default function ShelfPanel() {
    const [items, setItems] = useState<ShelfItem[]>([]);
    // True while files from another app are held over the panel.
    const [receiving, setReceiving] = useState(false);

    const refresh = useCallback(() => {
        listItems().then(setItems, console.error);
    }, []);

    useEffect(() => {
        refresh();

        // Files dragged from Finder or another app. Tauri gives their paths, which a page's own drop event does not.
        const listening = getCurrentWebview().onDragDropEvent(({payload}) => {
            if (isDraggingOut()) return;

            if (payload.type === "enter" || payload.type === "over") {
                setReceiving(true);
            } else {
                setReceiving(false);
                if (payload.type === "drop") addPaths(payload.paths).then(setItems, console.error);
            }
        });
        return () => {
            listening.then((stop) => stop());
        };
    }, [refresh]);

    return (
        <main className={receiving ? "shelf shelf--receiving" : "shelf"}>
            {/* The header moves the window when dragged. */}
            <header className="shelf_header" data-tauri-drag-region>
                <h1 className="shelf_title" data-tauri-drag-region>Shelf</h1>
                {items.length > 0 && <span className="shelf_count" data-tauri-drag-region>{items.length}</span>}
                <button className="shelf_button shelf_close" onClick={() => hidePanel().catch(console.error)}
                        aria-label="Hide the shelf" title="Hide the shelf">
                    <X size={14} aria-hidden/>
                </button>
            </header>

            {items.length === 0 ? (
                <div className="shelf_empty">
                    <Inbox size={28} strokeWidth={1.5} aria-hidden/>
                    <p>Drop files here</p>
                </div>
            ) : (
                <ul className="shelf_list">
                    {items.map((item) => (
                        <Row key={item.id} item={item} onMoved={refresh}
                             onRemove={() => removeItem(item.id).then(setItems, console.error)}/>
                    ))}
                </ul>
            )}
        </main>
    );
}

type RowProps = {
    item: ShelfItem;
    /** The file may have left its place: the drag ended on something that took it. */
    onMoved(): void;
    onRemove(): void;
};

function Row({item, onMoved, onRemove}: RowProps) {
    const drag = useDragOut(item.path, onMoved);
    const Icon = item.isDir ? Folder : File;

    return (
        <li className="shelf_row" title={item.path} {...drag}>
            <Icon className="shelf_icon" size={18} strokeWidth={1.5} aria-hidden/>
            <span className="shelf_name">{item.name}</span>
            <span className="shelf_size">{item.size === null ? "Folder" : formatSize(item.size)}</span>
            {/* Always visible, not only on hover: a panel that is not the active window may get no hover. */}
            <button className="shelf_button" onClick={onRemove}
                    aria-label={`Remove ${item.name} from the shelf`}
                    title="Remove from the shelf (the file stays where it is)">
                <X size={13} aria-hidden/>
            </button>
        </li>
    );
}
