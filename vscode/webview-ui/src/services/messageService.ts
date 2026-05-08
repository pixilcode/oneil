/**
 * Message service — bridges VS Code postMessage ↔ Jotai store.
 *
 * Called once at module load time (before React renders) so the message
 * listener is attached and the "ready" signal is sent as early as possible.
 * This avoids the timing gap where a message arrives before useEffect fires.
 */

import { createStore } from "jotai"
import { appStateAtom } from "../store/atoms"
import { getVsCodeApi } from "../vscode"
import type { ExtensionMessage } from "../types/messages"

export type Store = ReturnType<typeof createStore>

/**
 * Attaches the window message listener and posts "ready" to the extension.
 * Must be called with the same store instance passed to Jotai's `<Provider>`.
 */
export function initMessageService(store: Store): void {
    window.addEventListener("message", (event: MessageEvent<ExtensionMessage>) => {
        const msg = event.data
        if (msg.type === "loading") {
            store.set(appStateAtom, { status: "loading" })
        } else if (msg.type === "instanceTree") {
            store.set(appStateAtom, { status: "ready", data: msg.data })
        } else if (msg.type === "error") {
            store.set(appStateAtom, { status: "error", message: msg.message })
        }
    })

    // Tell the extension the webview is ready to receive data.
    getVsCodeApi().postMessage({ type: "ready" })
}
