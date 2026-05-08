import * as vscode from "vscode"
import { LanguageClient } from "vscode-languageclient/node"

/** The command name the LSP registers as an executeCommand handler. */
const INSTANCE_TREE_COMMAND = "oneil/instanceTree"

/**
 * Singleton panel instance; at most one rendered view is open at a time.
 */
let currentPanel: RenderedViewPanel | undefined

/**
 * Opens (or reveals) the rendered view for the given URI, fetching instance
 * tree data from the language server and posting it to the webview.
 *
 * If a panel for a different file is already open it is replaced.
 */
/**
 * Reloads the currently open rendered view, if any. No-op if no panel exists.
 */
export async function reloadRenderedView(): Promise<void> {
    if (currentPanel) {
        await currentPanel.refresh()
    }
}

export async function openRenderedView(
    uri: vscode.Uri,
    client: LanguageClient,
    context: vscode.ExtensionContext,
): Promise<void> {
    if (currentPanel) {
        // Webview already mounted — reveal and refresh immediately.
        currentPanel.reveal(uri)
        await currentPanel.refresh()
    } else {
        // New panel — refresh is triggered by the "ready" message from the
        // React app once it has mounted its message listener.
        currentPanel = new RenderedViewPanel(uri, client, context)
        currentPanel.onDispose(() => {
            currentPanel = undefined
        })
    }
}

// ── Panel class ───────────────────────────────────────────────────────────────

class RenderedViewPanel {
    private readonly panel: vscode.WebviewPanel
    private uri: vscode.Uri
    private readonly client: LanguageClient
    private readonly disposables: vscode.Disposable[] = []
    private disposed = false

    constructor(
        uri: vscode.Uri,
        client: LanguageClient,
        context: vscode.ExtensionContext,
    ) {
        this.uri = uri
        this.client = client

        const webviewDistUri = vscode.Uri.joinPath(context.extensionUri, "out", "webview-ui")

        this.panel = vscode.window.createWebviewPanel(
            "oneilRenderedView",
            `Oneil: ${basename(uri)}`,
            vscode.ViewColumn.Beside,
            {
                enableScripts: true,
                localResourceRoots: [webviewDistUri],
            },
        )

        this.panel.webview.html = appHtml(this.panel.webview, webviewDistUri)

        // Handle messages sent from the webview to the extension.
        this.panel.webview.onDidReceiveMessage(
            (message: unknown) => this.handleWebviewMessage(message),
            undefined,
            this.disposables,
        )

        // Refresh whenever the panel becomes visible (e.g. user switches back
        // to it, or VS Code recreates the webview context after it was hidden).
        this.panel.onDidChangeViewState(
            ({ webviewPanel }) => {
                if (webviewPanel.visible) {
                    void this.refresh()
                }
            },
            undefined,
            this.disposables,
        )

        // Follow the active editor — when the user switches to a different
        // Oneil file the panel re-targets and refreshes automatically.
        vscode.window.onDidChangeActiveTextEditor(
            (editor) => {
                if (editor?.document.languageId === "oneil") {
                    this.reveal(editor.document.uri)
                    void this.refresh()
                }
            },
            undefined,
            this.disposables,
        )

        // Refresh whenever any Oneil file is saved — saving a dependency can
        // change the evaluated tree for the root model too.
        vscode.workspace.onDidSaveTextDocument(
            (doc) => {
                if (doc.languageId === "oneil") {
                    void this.refresh()
                }
            },
            undefined,
            this.disposables,
        )

        this.panel.onDidDispose(() => this.dispose(), undefined, this.disposables)
    }

    /**
     * Reveals the panel. If the URI has changed the title is updated and data
     * is re-fetched on the next `refresh()` call.
     */
    reveal(uri: vscode.Uri): void {
        if (uri.toString() !== this.uri.toString()) {
            this.uri = uri
            this.panel.title = `Oneil: ${basename(uri)}`
        }
        this.panel.reveal()
    }

    /**
     * Fetches a fresh instance tree from the LSP and posts it to the webview.
     */
    async refresh(): Promise<void> {
        if (this.disposed) return

        this.panel.webview.postMessage({ type: "loading" })

        try {
            const tree = await this.client.sendRequest<unknown>(
                "workspace/executeCommand",
                {
                    command: INSTANCE_TREE_COMMAND,
                    arguments: [this.uri.toString()],
                },
            )
            if (this.disposed) return
            this.panel.webview.postMessage({ type: "instanceTree", data: tree })
        } catch (err) {
            if (this.disposed) return
            const message = err instanceof Error ? err.message : String(err)
            this.panel.webview.postMessage({ type: "error", message })
        }
    }

    /** Registers a callback for when the panel is disposed. */
    onDispose(cb: () => void): void {
        this.panel.onDidDispose(cb, undefined, this.disposables)
    }

    private handleWebviewMessage(message: unknown): void {
        if (typeof message === "object" && message !== null && "type" in message) {
            const msg = message as { type: string }
            if (msg.type === "ready") {
                // React app has mounted its listener — safe to send data now.
                this.refresh()
                return
            }
            if (msg.type === "reload") {
                void this.refresh()
                return
            }
        }
        // Future: sweep-parameter overrides and other webview→extension messages.
        console.log("[oneil] webview message:", message)
    }

    private dispose(): void {
        this.disposed = true
        for (const d of this.disposables) {
            d.dispose()
        }
        this.disposables.length = 0
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Returns the filename portion of a URI (e.g. `satellite.on`). */
function basename(uri: vscode.Uri): string {
    return uri.path.split("/").at(-1) ?? uri.path
}

/**
 * Returns the HTML shell that loads the built React bundle from
 * `out/webview-ui/assets/index.js`.
 *
 * The Content-Security-Policy allows only scripts from the extension's own
 * `out/webview-ui` directory (via the nonce-less `'self'`-equivalent webview
 * source scheme that VS Code requires).
 */
function appHtml(webview: vscode.Webview, distUri: vscode.Uri): string {
    const scriptUri = webview.asWebviewUri(
        vscode.Uri.joinPath(distUri, "assets", "index.js"),
    )
    const styleUri = webview.asWebviewUri(
        vscode.Uri.joinPath(distUri, "assets", "index.css"),
    )
    const nonce = getNonce()

    return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta http-equiv="Content-Security-Policy"
        content="default-src 'none';
                 style-src ${webview.cspSource} 'unsafe-inline';
                 script-src 'nonce-${nonce}';
                 font-src ${webview.cspSource};" />
  <title>Oneil Rendered View</title>
  <link rel="stylesheet" href="${styleUri}" />
</head>
<body>
  <div id="root"></div>
  <script type="module" nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`
}

/** Generates a cryptographically random nonce for use in the CSP. */
function getNonce(): string {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    return Array.from({ length: 32 }, () => chars[Math.floor(Math.random() * chars.length)]).join("")
}
