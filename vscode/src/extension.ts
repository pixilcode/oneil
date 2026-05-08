import * as vscode from "vscode"
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node"
import { openRenderedView, reloadRenderedView } from "./webview/panel"

let client: LanguageClient | undefined

export async function activate(context: vscode.ExtensionContext) {
    client?.info("starting language server")
    await restartLanguageServer(context)
    client?.info("language server started")

    client?.info("registering restart language server command")
    context.subscriptions.push(
        vscode.commands.registerCommand("oneil.restartLanguageServer", () =>
            restartLanguageServer(context),
        ),
    )
    client?.info("restart language server command registered")
    context.subscriptions.push(
        vscode.commands.registerCommand("oneil.openRenderedView", async () => {
            const activeUri = vscode.window.activeTextEditor?.document.uri
            if (!activeUri) {
                void vscode.window.showWarningMessage(
                    "Oneil: open an Oneil file before opening the rendered view.",
                )
                return
            }
            if (!client) {
                void vscode.window.showWarningMessage(
                    "Oneil: language server is not running.",
                )
                return
            }
            await openRenderedView(activeUri, client, context)
        }),
    )
    context.subscriptions.push(
        vscode.commands.registerCommand("oneil.reloadRenderedView", () =>
            reloadRenderedView(),
        ),
    )
    client?.info("extension is now active!")
}

export function deactivate(): Thenable<void> | undefined {
    return client?.stop()
}

/**
 * Builds server and client options from the current Oneil configuration.
 */
function buildOptions(): { serverOptions: ServerOptions; clientOptions: LanguageClientOptions } {
    const config = vscode.workspace.getConfiguration("oneil")
    const configuredPath = config.get<string | null>("serverPath", null)
    const command = configuredPath ?? process.env.ONEIL_PATH ?? "oneil"

    return {
        serverOptions: { command, args: ["lsp"] },
        clientOptions: {
            documentSelector: [
                { scheme: "file", language: "oneil" },
                { scheme: "file", language: "python" },
            ],
        },
    }
}

/**
 * Restarts the Oneil language server. Uses the current configuration (e.g. serverPath).
 */
async function restartLanguageServer(context: vscode.ExtensionContext): Promise<void> {
    if (client == null) {
        const { serverOptions, clientOptions } = buildOptions()

        const newClient = new LanguageClient(
            "oneil-language-server",
            "Oneil Language Server",
            serverOptions,
            clientOptions,
        )
        await newClient.start()

        client = newClient
        client.info("language server initialized")
    } else {
        client.info("restarting language server")
        await client.restart()
        client.info("language server restarted")
    }
}
