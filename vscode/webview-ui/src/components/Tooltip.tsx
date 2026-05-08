import { useAtomValue } from "jotai"
import { createContext, useCallback, useContext, useState, type ReactNode } from "react"
import { createPortal } from "react-dom"
import { fontScaleAtom, graphZoomAtom } from "../store/atoms"
import { NoteDisplay } from "../views/InstanceTree"

interface TooltipState {
    content: string | null
    x: number
    y: number
}

interface TooltipContextValue {
    show: (content: string, x: number, y: number) => void
    hide: () => void
}

const TooltipContext = createContext<TooltipContextValue | null>(null)

/**
 * Provides tooltip functionality to the component tree.
 * Renders the tooltip via portal at document body level so it appears
 * above all other content including ReactFlow nodes.
 * Font size scales with both the user's fontScale preference and the
 * current ReactFlow zoom level.
 */
export function TooltipProvider({ children }: { children: ReactNode }) {
    const [state, setState] = useState<TooltipState>({ content: null, x: 0, y: 0 })
    const fontScale = useAtomValue(fontScaleAtom)
    const graphZoom = useAtomValue(graphZoomAtom)

    const show = useCallback((content: string, x: number, y: number) => {
        setState({ content, x, y })
    }, [])

    const hide = useCallback(() => {
        setState((s) => ({ ...s, content: null }))
    }, [])

    // Combine font scale and graph zoom; clamp zoom contribution to reasonable bounds
    const effectiveZoom = Math.max(0.5, Math.min(2, graphZoom))
    const tooltipFontSize = fontScale * 0.85 * effectiveZoom

    return (
        <TooltipContext.Provider value={{ show, hide }}>
            {children}
            {state.content && createPortal(
                <div
                    className="tooltip-popup"
                    style={{
                        position: "fixed",
                        left: state.x,
                        top: state.y,
                        fontSize: `calc(var(--vscode-font-size, 13px) * ${tooltipFontSize})`,
                    }}
                >
                    <NoteDisplay text={state.content} />
                </div>,
                document.body,
            )}
        </TooltipContext.Provider>
    )
}

/**
 * Hook to access tooltip show/hide functions.
 */
export function useTooltip() {
    const ctx = useContext(TooltipContext)
    if (!ctx) throw new Error("useTooltip must be used within TooltipProvider")
    return ctx
}

/**
 * Props for creating tooltip trigger behavior.
 * Spread these onto an element to make it show a tooltip on hover.
 */
export function useTooltipTrigger(content: string | null | undefined) {
    const tooltip = useTooltip()

    const onMouseEnter = useCallback(
        (e: React.MouseEvent) => {
            if (!content) return
            const rect = e.currentTarget.getBoundingClientRect()
            tooltip.show(content, rect.left, rect.bottom + 4)
        },
        [content, tooltip],
    )

    const onMouseLeave = useCallback(() => {
        tooltip.hide()
    }, [tooltip])

    if (!content) return {}

    return {
        onMouseEnter,
        onMouseLeave,
        className: "has-tooltip",
    }
}
