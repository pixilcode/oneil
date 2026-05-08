import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { resolve } from "path";
// https://vite.dev/config/
export default defineConfig({
    plugins: [react()],
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
                assetFileNames: "assets/[name].[ext]",
            },
        },
    },
});
//# sourceMappingURL=vite.config.js.map