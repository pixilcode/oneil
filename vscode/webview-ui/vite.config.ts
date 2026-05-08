import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import { resolve } from "path"

// https://vite.dev/config/
export default defineConfig({
    plugins: [react()],
    // Use relative asset paths so KaTeX fonts resolve correctly inside the
    // VS Code webview, which doesn't have a real web server root.
    base: "./",
    build: {
        // Output into the extension's out directory so the panel can load it.
        outDir: resolve(__dirname, "../out/webview-ui"),
        emptyOutDir: true,
        rollupOptions: {
            input: resolve(__dirname, "index.html"),
            output: {
                // Single deterministic filenames — the panel HTML references
                // these exact paths via vscode.Uri.joinPath.
                entryFileNames: "assets/index.js",
                chunkFileNames: "assets/[name].js",
                // All CSS is collected into one deterministic file so the
                // webview HTML can reference it at a known path.
                assetFileNames: (info) =>
                    info.name?.endsWith(".css") ? "assets/index.css" : "assets/[name].[ext]",
            },
        },
    },
})
