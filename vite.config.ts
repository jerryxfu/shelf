import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";

// Set by the Tauri CLI when the dev server has to be reached from another device (mobile). Unused on desktop.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
    plugins: [react()],
    // Keep Rust's errors on screen: Vite would clear them.
    clearScreen: false,
    server: {
        // Must match devUrl in src-tauri/tauri.conf.json. Tauri expects a fixed port, so fail if it is taken.
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host ? {protocol: "ws", host, port: 1421} : undefined,
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
    envPrefix: ["VITE_", "TAURI_ENV_*"],
    build: {
        // The system web views: WebView2 (Chromium) on Windows, WKWebView (Safari) on macOS.
        target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
        minify: !process.env.TAURI_ENV_DEBUG,
        sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
});
