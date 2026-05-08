import { createStore, Provider, useAtom, useAtomValue } from "jotai"
import { useState } from "react"
import { TooltipProvider } from "./components/Tooltip"
import { initMessageService } from "./services/messageService"
import {
    appStateAtom,
    FONT_SCALE_MAX,
    FONT_SCALE_MIN,
    FONT_SCALE_STEP,
    fontScaleAtom,
    showDesignsAtom,
    showNotesAtom,
    showTraceAtom,
} from "./store/atoms"
import type { RenderedNode } from "./types/model"
import { getVsCodeApi } from "./vscode"
import { InstanceTreeView } from "./views/InstanceTree"
import { ModelGraphView } from "./views/ModelGraph"

/**
 * Single Jotai store — analogous to Riverpod's ProviderScope.
 * Created at module load so atom values survive React StrictMode remounts.
 */
const store = createStore()

/**
 * Attach the VS Code message listener and send "ready" before React renders,
 * so no messages can arrive before the listener is active.
 */
initMessageService(store)

export function App() {
    return (
        <Provider store={store}>
            <TooltipProvider>
                <AppContent />
            </TooltipProvider>
        </Provider>
    )
}

type ViewMode = "tree" | "graph"

/** Routes to the correct view based on current app state. */
function AppContent() {
    const state = useAtomValue(appStateAtom)
    const [view, setView] = useState<ViewMode>("tree")
    const fontScale = useAtomValue(fontScaleAtom)

    // Determine if any node in the tree has applied designs so we know
    // whether to show the "Show Designs" toggle at all.
    const hasDesigns =
        state.status === "ready" && treeHasDesigns(state.data.root)

    return (
        <div
            className="app"
            style={{ fontSize: `calc(var(--vscode-font-size, 13px) * ${fontScale})` }}
        >
            <Toolbar view={view} onViewChange={setView} hasDesigns={hasDesigns} />
            {state.status === "loading" && <p className="status">Loading…</p>}
            {state.status === "error" && <p className="status error">Error: {state.message}</p>}
            {state.status === "ready" && view === "tree" && (
                <InstanceTreeView node={state.data.root} referencePool={state.data.reference_pool} />
            )}
            {state.status === "ready" && view === "graph" && (
                <ModelGraphView node={state.data.root} referencePool={state.data.reference_pool} />
            )}
        </div>
    )
}

/** Recursively checks whether any node in the tree has applied designs. */
function treeHasDesigns(node: RenderedNode): boolean {
    if (node.applied_designs.length > 0) return true
    return node.children.some((c) => treeHasDesigns(c.node))
}

interface ToolbarProps {
    view: ViewMode
    onViewChange: (v: ViewMode) => void
    hasDesigns: boolean
}

/** Tab bar. Includes a "Show Designs" toggle when the model has applied designs.
 * In dev builds also shows a reload button.
 */
function Toolbar({ view, onViewChange, hasDesigns }: ToolbarProps) {
    const [showDesigns, setShowDesigns] = useAtom(showDesignsAtom)
    const [showNotes, setShowNotes] = useAtom(showNotesAtom)
    const [showTrace, setShowTrace] = useAtom(showTraceAtom)
    const [fontScale, setFontScale] = useAtom(fontScaleAtom)

    const decreaseFontSize = () =>
        setFontScale((s) => Math.max(FONT_SCALE_MIN, Math.round((s - FONT_SCALE_STEP) * 10) / 10))
    const increaseFontSize = () =>
        setFontScale((s) => Math.min(FONT_SCALE_MAX, Math.round((s + FONT_SCALE_STEP) * 10) / 10))

    return (
        <div className="toolbar">
            <button
                className={`toolbar-btn${view === "tree" ? " active" : ""}`}
                onClick={() => onViewChange("tree")}
            >
                Tree
            </button>
            <button
                className={`toolbar-btn${view === "graph" ? " active" : ""}`}
                onClick={() => onViewChange("graph")}
            >
                Graph
            </button>
            {hasDesigns && (
                <label className="toolbar-toggle" title="Highlight design additions and overrides">
                    <input
                        type="checkbox"
                        checked={showDesigns}
                        onChange={(e) => setShowDesigns(e.target.checked)}
                    />
                    Designs
                </label>
            )}
            <label className="toolbar-toggle" title="Show notes inline (hover labels when off)">
                <input
                    type="checkbox"
                    checked={showNotes}
                    onChange={(e) => setShowNotes(e.target.checked)}
                />
                Notes
            </label>
            <label className="toolbar-toggle" title="Show trace/debug parameters">
                <input
                    type="checkbox"
                    checked={showTrace}
                    onChange={(e) => setShowTrace(e.target.checked)}
                />
                Trace
            </label>
            <div className="toolbar-font-size" title="Adjust font size">
                <button
                    className="toolbar-btn"
                    onClick={decreaseFontSize}
                    disabled={fontScale <= FONT_SCALE_MIN}
                    aria-label="Decrease font size"
                >
                    A−
                </button>
                <span className="toolbar-font-label">{Math.round(fontScale * 100)}%</span>
                <button
                    className="toolbar-btn"
                    onClick={increaseFontSize}
                    disabled={fontScale >= FONT_SCALE_MAX}
                    aria-label="Increase font size"
                >
                    A+
                </button>
            </div>
            {import.meta.env.DEV && (
                <button
                    className="toolbar-btn"
                    onClick={() => getVsCodeApi().postMessage({ type: "reload" })}
                    title="Reload rendered view"
                >
                    ↺
                </button>
            )}
        </div>
    )
}
