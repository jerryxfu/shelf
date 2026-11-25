import {StrictMode} from "react";
import {createRoot} from "react-dom/client";

import "./index.scss";
import ShelfPanel from "./shelf/ShelfPanel.tsx";

// macOS puts its own translucent material behind the page; elsewhere the page paints its background (index.scss).
document.documentElement.dataset.platform = navigator.userAgent.includes("Mac OS X") ? "macos" : "other";

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <ShelfPanel/>
    </StrictMode>,
);
