import { useAtom, useAtomValue, useSetAtom } from "jotai"
import katex from "katex"
import { createContext, useCallback, useContext, useEffect, useMemo } from "react"
import { useTooltipTrigger } from "../components/Tooltip"
import type { AppliedDesign, ParameterValueAst, RenderedChild, RenderedNode, RenderedParameter, RenderedPoolEntry, RenderedValue } from "../types/model"
import { graphZoomAtom, highlightedDepsAtom, showDesignsAtom, showNotesAtom, showTraceAtom } from "../store/atoms"
import { designColorVar, modelDisplayName } from "../utils/designColors"
import { buildAliasToModelPath, extractDependencyKeys, mainTreeParamKey, refPoolParamKey } from "../utils/extractDependencies"
import { isSimpleLiteral, paramExprOnlyToLatex } from "../utils/exprToLatex"

// ── Context for dependency highlighting ──────────────────────────────────────

interface DepContext {
    /** Maps reference alias → model_path for resolving external references */
    aliasToModelPath: Map<string, string>
    /** "main" for main tree, or model_path for reference pool entries */
    treeType: "main" | { modelPath: string }
}

const DepHighlightContext = createContext<DepContext>({
    aliasToModelPath: new Map(),
    treeType: "main",
})

// ── InstanceTreeView ──────────────────────────────────────────────────────────

interface InstanceTreeViewProps {
    node: RenderedNode
    referencePool: RenderedPoolEntry[]
}

/**
 * Renders the full evaluated instance tree rooted at `node`,
 * plus any models in the reference pool.
 */
export function InstanceTreeView({ node, referencePool }: InstanceTreeViewProps) {
    // Reset graph zoom to 1 so tooltips in tree view aren't scaled
    const setGraphZoom = useSetAtom(graphZoomAtom)
    useEffect(() => {
        setGraphZoom(1)
    }, [setGraphZoom])

    // Build alias → model_path mapping once for the whole tree
    const aliasToModelPath = useMemo(() => buildAliasToModelPath(node), [node])

    const mainCtx: DepContext = useMemo(
        () => ({ aliasToModelPath, treeType: "main" }),
        [aliasToModelPath],
    )

    return (
        <div className="instance-tree">
            <DepHighlightContext.Provider value={mainCtx}>
                <ModelNode node={node} depth={0} />
            </DepHighlightContext.Provider>
            {referencePool.length > 0 && (
                <>
                    <h2 className="reference-pool-header">Reference Imports</h2>
                    {referencePool.map((entry) => (
                        <RefPoolEntry
                            key={entry.alias}
                            entry={entry}
                            aliasToModelPath={aliasToModelPath}
                        />
                    ))}
                </>
            )}
        </div>
    )
}

function RefPoolEntry({
    entry,
    aliasToModelPath,
}: {
    entry: RenderedPoolEntry
    aliasToModelPath: Map<string, string>
}) {
    const ctx: DepContext = useMemo(
        () => ({ aliasToModelPath, treeType: { modelPath: entry.node.model_path } }),
        [aliasToModelPath, entry.node.model_path],
    )

    return (
        <div className="reference-pool-entry">
            <span className="reference-alias">{entry.alias}</span>
            <DepHighlightContext.Provider value={ctx}>
                <ModelNode node={entry.node} depth={0} />
            </DepHighlightContext.Provider>
        </div>
    )
}

// ── ModelNode ─────────────────────────────────────────────────────────────────

interface ModelNodeProps {
    node: RenderedNode
    depth: number
}

function ModelNode({ node, depth }: ModelNodeProps) {
    const indent = depth * 1.5
    const modelName = modelDisplayName(node.model_path)
    const showNotes = useAtomValue(showNotesAtom)
    const tooltipProps = useTooltipTrigger(!showNotes ? node.note : undefined)

    // Build a fast lookup from design_name → color_index for this node.
    const designIndex = buildDesignIndex(node.applied_designs)

    return (
        <section style={{ marginLeft: `${indent}rem` }}>
            <div className="model-heading-row">
                <h3
                    className={`model-heading${tooltipProps.className ? ` ${tooltipProps.className}` : ""}`}
                    onMouseEnter={tooltipProps.onMouseEnter}
                    onMouseLeave={tooltipProps.onMouseLeave}
                >
                    {modelName}
                </h3>
                {node.applied_designs.map((d) => (
                    <DesignBadge key={d.design_name} design={d} />
                ))}
            </div>
            {showNotes && node.note && (
                <p className="model-note">
                    <NoteDisplay text={node.note} />
                </p>
            )}
            <ParameterList
                parameters={node.parameters}
                designIndex={designIndex}
                instancePath={node.instance_path}
            />
            {node.children.map((child) => (
                <ChildNode key={child.alias} child={child} depth={depth + 1} />
            ))}
        </section>
    )
}

// ── DesignBadge ───────────────────────────────────────────────────────────────

function DesignBadge({ design }: { design: AppliedDesign }) {
    return (
        <span
            className="design-badge"
            style={{ color: designColorVar(design.color_index), borderColor: designColorVar(design.color_index) }}
        >
            {design.design_name}
        </span>
    )
}

// ── ChildNode ─────────────────────────────────────────────────────────────────

function ChildNode({ child, depth }: { child: RenderedChild; depth: number }) {
    return (
        <div className="child-node">
            <span className="child-alias">{child.alias}</span>
            <ModelNode node={child.node} depth={depth} />
        </div>
    )
}

// ── ParameterList ─────────────────────────────────────────────────────────────

function ParameterList({
    parameters,
    designIndex,
    instancePath,
}: {
    parameters: RenderedParameter[]
    designIndex: Map<string, number>
    instancePath: string[]
}) {
    const showTrace = useAtomValue(showTraceAtom)
    const visibleParams = showTrace
        ? parameters
        : parameters.filter((p) => p.print_level !== "trace")

    if (visibleParams.length === 0) return null
    return (
        <ul className="parameter-list">
            {visibleParams.map((p) => (
                <ParameterRow
                    key={p.name}
                    param={p}
                    designIndex={designIndex}
                    instancePath={instancePath}
                />
            ))}
        </ul>
    )
}

// ── ParameterRow ──────────────────────────────────────────────────────────────

function ParameterRow({
    param,
    designIndex,
    instancePath,
}: {
    param: RenderedParameter
    designIndex: Map<string, number>
    instancePath: string[]
}) {
    const showDesigns = useAtomValue(showDesignsAtom)
    const showNotes = useAtomValue(showNotesAtom)
    const [highlightedDeps, setHighlightedDeps] = useAtom(highlightedDepsAtom)
    const { aliasToModelPath, treeType } = useContext(DepHighlightContext)
    const tooltipProps = useTooltipTrigger(!showNotes ? param.note : undefined)
    const mark = param.design
    const colorIdx = mark != null ? (designIndex.get(mark.design_name) ?? 0) : null

    // Extract dependency keys for hover highlighting
    const depKeys = useMemo(
        () => extractDependencyKeys(param.expression as ParameterValueAst | null, instancePath, aliasToModelPath),
        [param.expression, instancePath, aliasToModelPath],
    )

    // Compute this parameter's key for highlight checking
    const myKey = useMemo(() => {
        if (treeType === "main") {
            return mainTreeParamKey(instancePath, param.name)
        }
        return refPoolParamKey(treeType.modelPath, param.name)
    }, [treeType, instancePath, param.name])

    // Check if this parameter should be highlighted (it's a dependency of the hovered param)
    const isHighlighted = highlightedDeps.has(myKey)

    // Hover handlers for the param-name to highlight dependencies
    const onNameMouseEnter = useCallback(() => {
        if (depKeys.size > 0) {
            setHighlightedDeps(depKeys)
        }
    }, [depKeys, setHighlightedDeps])

    const onNameMouseLeave = useCallback(() => {
        setHighlightedDeps(new Set<string>())
    }, [setHighlightedDeps])

    const labelStyle: React.CSSProperties =
        showDesigns && mark != null && colorIdx != null
            ? {
                  borderLeftColor: designColorVar(colorIdx),
                  borderLeftWidth: "var(--design-border-width)",
                  borderLeftStyle: "solid",
                  paddingLeft: "6px",
                  ...(mark.is_addition
                      ? {
                            backgroundColor: `color-mix(in srgb, ${designColorVar(colorIdx)} var(--design-color-tint-alpha), transparent)`,
                        }
                      : {}),
              }
            : {}

    const showExpr =
        param.expression != null &&
        !isSimpleLiteral(param.expression as ParameterValueAst)
    const isPerformance = param.print_level === "performance"

    const rowClasses = [
        "parameter-row",
        isPerformance && "param-performance",
        isHighlighted && "param-highlighted",
    ]
        .filter(Boolean)
        .join(" ")

    return (
        <li className={rowClasses}>
            <span
                className={`param-label${tooltipProps.className ? ` ${tooltipProps.className}` : ""}`}
                style={labelStyle}
                onMouseEnter={tooltipProps.onMouseEnter}
                onMouseLeave={tooltipProps.onMouseLeave}
            >
                {param.label}
            </span>
            {showExpr ? (
                <span className="param-expr">
                    <ExprDisplay expr={param.expression as ParameterValueAst} />
                </span>
            ) : (
                <span className="param-expr" />
            )}
            <span className="param-sep">:</span>
            <span className="param-value">
                <span
                    className={`param-name${depKeys.size > 0 ? " has-deps" : ""}`}
                    onMouseEnter={onNameMouseEnter}
                    onMouseLeave={onNameMouseLeave}
                >
                    {param.name}
                </span>
                {" = "}
                <ValueDisplay value={param.value} />
            </span>
            {showNotes && param.note && (
                <div className="param-note-line">
                    <NoteDisplay text={param.note} />
                </div>
            )}
        </li>
    )
}

// ── ExprDisplay ───────────────────────────────────────────────────────────────

/**
 * Renders a `ParameterValueAst` using KaTeX in MathML mode (no external fonts
 * needed — VS Code webviews run Chromium which supports MathML natively).
 */
function ExprDisplay({ expr }: { expr: ParameterValueAst }) {
    try {
        const latex = paramExprOnlyToLatex(expr)
        const html = katex.renderToString(latex, {
            output: "mathml",
            throwOnError: false,
        })
        return <span dangerouslySetInnerHTML={{ __html: html }} />
    } catch {
        return null
    }
}

// ── NoteDisplay ───────────────────────────────────────────────────────────────

/**
 * Renders an Oneil note string as rich content supporting:
 *   - `\begin{equation}...\end{equation}` → KaTeX display math
 *   - `$$...$$`                           → KaTeX display math
 *   - `$...$`                             → KaTeX inline math
 *   - `\cite{key}`                        → [key] badge
 *   - `\ref{label}` / `\label{label}`     → suppressed or [ref]
 *   - `~`                                 → non-breaking space
 *   - `**bold**`, `*italic*`, `` `code` `` → HTML equivalents
 */
export function NoteDisplay({ text }: { text: string }) {
    const parts = parseNoteText(text)
    return (
        <span className="note-content">
            {parts.map((part, i) => {
                if (part.type === "math-block") {
                    try {
                        const html = katex.renderToString(part.src, {
                            output: "mathml",
                            throwOnError: false,
                            displayMode: true,
                        })
                        return (
                            <span
                                key={i}
                                className="note-math-block"
                                dangerouslySetInnerHTML={{ __html: html }}
                            />
                        )
                    } catch {
                        return <code key={i}>{part.src}</code>
                    }
                }
                if (part.type === "math-inline") {
                    try {
                        const html = katex.renderToString(part.src, {
                            output: "mathml",
                            throwOnError: false,
                        })
                        return <span key={i} dangerouslySetInnerHTML={{ __html: html }} />
                    } catch {
                        return <code key={i}>{part.src}</code>
                    }
                }
                if (part.type === "cite") {
                    return (
                        <span key={i} className="note-cite" title={part.src}>
                            [{part.src}]
                        </span>
                    )
                }
                // plain text — apply basic inline markdown
                return <span key={i} dangerouslySetInnerHTML={{ __html: markdownInline(part.src) }} />
            })}
        </span>
    )
}

type NotePart =
    | { type: "text"; src: string }
    | { type: "math-inline"; src: string }
    | { type: "math-block"; src: string }
    | { type: "cite"; src: string }

/**
 * Splits note text into typed segments.  Handles, in order of priority:
 *   1. `\begin{equation}...\end{equation}` blocks → display math
 *      (strips nested `\label{...}` which KaTeX does not support)
 *   2. `$$...$$` → display math
 *   3. `$...$`   → inline math
 *   4. `\cite{key}` → cite badge
 *   5. Everything else → plain text (with `~`, `\ref{}`, `\label{}` handled by
 *      `markdownInline`)
 */
function parseNoteText(text: string): NotePart[] {
    const parts: NotePart[] = []
    // Matches, in priority order, using alternation.
    const re = /\\begin\{equation\}([\s\S]*?)\\end\{equation\}|\$\$([^$]+)\$\$|\$([^$\n]+)\$|\\cite\{([^}]+)\}/g
    let last = 0
    for (const m of text.matchAll(re)) {
        if (m.index! > last) parts.push({ type: "text", src: text.slice(last, m.index) })
        if (m[1] !== undefined) {
            // \begin{equation}...\end{equation}: strip \label{} for KaTeX
            const body = m[1].replace(/\\label\{[^}]*\}/g, "").trim()
            parts.push({ type: "math-block", src: body })
        } else if (m[2] !== undefined) {
            parts.push({ type: "math-block", src: m[2] })
        } else if (m[3] !== undefined) {
            parts.push({ type: "math-inline", src: m[3] })
        } else {
            parts.push({ type: "cite", src: m[4] })
        }
        last = m.index! + m[0].length
    }
    if (last < text.length) parts.push({ type: "text", src: text.slice(last) })
    return parts
}

/** Applies inline markdown and strips/replaces remaining LaTeX macros. */
function markdownInline(raw: string): string {
    return raw
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        // LaTeX non-breaking space → real NBSP
        .replace(/~/g, "\u00a0")
        // \ref{label} and \label{label} → suppressed (cross-ref not resolvable here)
        .replace(/\\(?:ref|label)\{[^}]*\}/g, "")
        // Common text macros
        .replace(/\\textdegree\b/g, "°")
        .replace(/\\emph\{([^}]*)\}/g, "<em>$1</em>")
        .replace(/\\textbf\{([^}]*)\}/g, "<strong>$1</strong>")
        .replace(/\\texttt\{([^}]*)\}/g, "<code>$1</code>")
        // Markdown
        .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
        .replace(/\*(.+?)\*/g, "<em>$1</em>")
        .replace(/`(.+?)`/g, "<code>$1</code>")
}

// ── ValueDisplay ──────────────────────────────────────────────────────────────

export function ValueDisplay({ value }: { value: RenderedValue }) {
    switch (value.type) {
        case "boolean":
            return <>{String(value.value)}</>
        case "string":
            return <>&ldquo;{value.value}&rdquo;</>
        case "number":
            return value.max !== null
                ? <>[{fmt(value.value)}, {fmt(value.max)}]</>
                : <>{fmt(value.value)}</>
        case "measured_number":
            return value.max !== null
                ? <>[{fmt(value.value)}, {fmt(value.max)}] {value.unit}</>
                : <>{fmt(value.value)} {value.unit}</>
    }
}

/**
 * Formats a number to at most 4 significant figures, trimming trailing zeros
 * so that `9.810` → `"9.81"` and `5.97e24` → `"5.97e+24"`.
 */
function fmt(raw: number | string): string {
    const num = typeof raw === "number" ? raw : parseFloat(raw)
    if (!isFinite(num)) return String(raw)
    const s = num.toPrecision(4)
    if (s.includes("e")) {
        // Strip trailing zeros from mantissa: "5.970e+24" → "5.97e+24"
        return s.replace(/(\.\d*?)0+(e)/, "$1$2").replace(/\.(e)/, "$1")
    }
    // Strip trailing zeros after decimal: "9.810" → "9.81", "1.000" → "1"
    return s.replace(/(\.\d*?)0+$/, "$1").replace(/\.$/, "")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Builds a `design_name → color_index` map from a node's applied_designs. */
function buildDesignIndex(designs: AppliedDesign[]): Map<string, number> {
    return new Map(designs.map((d) => [d.design_name, d.color_index]))
}
