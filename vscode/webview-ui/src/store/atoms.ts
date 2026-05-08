/**
 * Jotai atoms — the single source of truth for all app state.
 *
 * Philosophy (analogous to Riverpod):
 *   - Each atom is like a Riverpod provider: a named, typed piece of state.
 *   - Derived atoms (atom(get => ...)) replace computed providers.
 *   - Components subscribe with useAtomValue / useAtom; the store can be
 *     written from outside React via store.set() (used by the message service).
 */

import { atom } from "jotai"
import { atomWithStorage } from "jotai/utils"
import type { RenderedTree } from "../types/model"

// ── App-level state ───────────────────────────────────────────────────────────

export type AppState =
    | { status: "loading" }
    | { status: "ready"; data: RenderedTree }
    | { status: "error"; message: string }

/** The current evaluation result from the language server. */
export const appStateAtom = atom<AppState>({ status: "loading" })

// ── UI preference atoms ───────────────────────────────────────────────────────

/**
 * Whether design additions/overrides are visually highlighted.
 * When `false`, all parameters render identically regardless of provenance.
 */
export const showDesignsAtom = atom(true)

/**
 * Whether notes are displayed inline in the tree and graph views.
 * When `false`, notes are hidden but accessible via hover tooltip on the label.
 */
export const showNotesAtom = atom(false)

/**
 * Whether trace/debug parameters are shown.
 * When `false`, parameters with `print_level === "trace"` are hidden.
 */
export const showTraceAtom = atom(false)

/**
 * User-controlled font scale multiplier applied on top of VS Code's base font
 * size. A value of 1.5 means the webview renders at 1.5× the editor font size,
 * which is the default because nested KaTeX subscripts otherwise become too
 * small to read comfortably. Persisted to localStorage so the preference
 * survives panel reloads.
 */
export const FONT_SCALE_MIN = 0.8
export const FONT_SCALE_MAX = 3.0
export const FONT_SCALE_STEP = 0.1
export const fontScaleAtom = atomWithStorage("oneil.fontScale", 1.5)

// ── Derived atoms (add as the UI grows) ───────────────────────────────────────

/** True while waiting for the LSP response. */
export const isLoadingAtom = atom((get) => get(appStateAtom).status === "loading")

/** The main instance tree root, or null when not yet loaded. */
export const instanceTreeAtom = atom((get) => {
    const s = get(appStateAtom)
    return s.status === "ready" ? s.data.root : null
})

/** The reference pool (external models), or empty array when not loaded. */
export const referencePoolAtom = atom((get) => {
    const s = get(appStateAtom)
    return s.status === "ready" ? s.data.reference_pool : []
})

/** Current ReactFlow viewport zoom level (1 = 100%). Used for scaling tooltips in graph view. */
export const graphZoomAtom = atom(1)

/**
 * Currently hovered parameter for dependency highlighting.
 * When set, parameters that are dependencies of this one are highlighted.
 * Format: `{ modelPath: string, paramName: string }` or null.
 */
export const hoveredParamAtom = atom<{ modelPath: string; paramName: string } | null>(null)

/**
 * Set of parameter keys that should be highlighted as dependencies.
 * Format: "local:paramName" or "external:refName.paramName"
 */
export const highlightedDepsAtom = atom<Set<string>>(new Set<string>())
