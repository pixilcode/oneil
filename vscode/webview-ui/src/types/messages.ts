/**
 * Messages the VS Code extension posts into the webview, and messages the
 * webview posts back to the extension.
 *
 * Keep in sync with the TypeScript in `vscode/src/webview/panel.ts`.
 */

import type { RenderedTree } from "./model"

// ── Extension → webview ───────────────────────────────────────────────────────

export type ExtensionMessage =
    | { type: "loading" }
    | { type: "instanceTree"; data: RenderedTree }
    | { type: "error"; message: string }

// ── Webview → extension ───────────────────────────────────────────────────────

export type WebviewMessage =
    /** Sent once on mount so the extension knows the webview is ready. */
    | { type: "ready" }
    /** Requests the extension to re-fetch and push a fresh instance tree. */
    | { type: "reload" }
