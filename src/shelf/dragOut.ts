// Dragging a shelf item out of the panel, into Finder or another app.
//
// A web page cannot start a drag of real files by itself, so the drag is handed to the system by
// tauri-plugin-drag. It starts from pointer events, not from the page's own `dragstart`: Tauri's handler for files
// dropped on the panel turns the page's drag and drop off on Windows.

import {type PointerEvent, useRef} from "react";
import {startDrag} from "@crabnebula/tauri-plugin-drag";

/** How far the pointer moves, in pixels, before a press becomes a drag. */
const THRESHOLD = 4;

let draggingOut = false;

/** True while one of the shelf's own files is being dragged: the panel then ignores it passing over itself. */
export const isDraggingOut = () => draggingOut;

/**
 * The picture that follows the pointer. The plugin wants a PNG; this one is a plain sheet of paper, drawn once.
 * A placeholder: the file's real icon would have to come from the core.
 */
let preview: string | undefined;

function previewImage(): string {
    if (preview) return preview;

    const canvas = document.createElement("canvas");
    canvas.width = 40;
    canvas.height = 48;
    const pen = canvas.getContext("2d")!;
    pen.fillStyle = "#ffffff";
    pen.strokeStyle = "#8e8e93";
    pen.lineWidth = 1.5;
    pen.beginPath();
    pen.roundRect(4, 2, 32, 44, 5);
    pen.fill();
    pen.stroke();
    pen.fillStyle = "#c7c7cc";
    for (const y of [14, 22, 30]) pen.fillRect(11, y, 18, 2.5);

    preview = canvas.toDataURL("image/png");
    return preview;
}

/**
 * Pointer handlers that make an element draggable out of the panel as the file at `path`.
 *
 * @param onDropped called once the file has been dropped somewhere that took it
 */
export function useDragOut(path: string, onDropped: () => void) {
    const pressedAt = useRef<{x: number; y: number} | null>(null);

    return {
        onPointerDown(event: PointerEvent<HTMLElement>) {
            // The main button only, and not on the element's own buttons.
            if (event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
            pressedAt.current = {x: event.clientX, y: event.clientY};
        },
        onPointerMove(event: PointerEvent<HTMLElement>) {
            const from = pressedAt.current;
            if (!from || Math.hypot(event.clientX - from.x, event.clientY - from.y) < THRESHOLD) return;

            pressedAt.current = null;
            draggingOut = true;
            // "copy": the place it is dropped gets a copy and the file stays where it was. The plugin offers copy or
            // move, not both as Finder does.
            startDrag({item: [path], icon: previewImage(), mode: "copy"}, ({result}) => {
                draggingOut = false;
                if (result === "Dropped") onDropped();
            }).catch((error: unknown) => {
                draggingOut = false;
                console.error("The drag could not start:", error);
            });
        },
        onPointerUp() {
            pressedAt.current = null;
        },
        onPointerCancel() {
            pressedAt.current = null;
        },
    };
}
