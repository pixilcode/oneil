import { useAtom, useAtomValue, useSetAtom } from "jotai"
import katex from "katex"
import { createContext, useCallback, useContext, useMemo } from "react"
import ReactFlow, {
    type Node,
    type Viewport,
    Controls,
    Background,
    BackgroundVariant,
    Handle,
    Position,
} from "reactflow"
import "reactflow/dist/style.css"
import { useTooltipTrigger } from "../components/Tooltip"
import type { AppliedDesign, ParameterValueAst, RenderedNode, RenderedParameter, RenderedPoolEntry } from "../types/model"
import { graphZoomAtom, highlightedDepsAtom, showDesignsAtom, showNotesAtom, showTraceAtom } from "../store/atoms"
import { designColorVar, modelDisplayName } from "../utils/designColors"
import { buildAliasToModelPath, extractDependencyKeys, mainTreeParamKey, refPoolParamKey } from "../utils/extractDependencies"
import { isSimpleLiteral, paramExprOnlyToLatex } from "../utils/exprToLatex"
import { type ContentSize, type MeasureItem, useMeasureContent } from "../utils/measureContent"
import { NoteDisplay, ValueDisplay } from "./InstanceTree"

// ── Context for dependency highlighting ──────────────────────────────────────

interface GraphDepContext {
    aliasToModelPath: Map<string, string>
    treeType: "main" | { modelPath: string }
}

const GraphDepContext = createContext<GraphDepContext>({
    aliasToModelPath: new Map(),
    treeType: "main",
})

// ── Layout constants (fallback estimates used before measurement arrives) ─────

/** Horizontal gap between sibling nodes inside a group. */
const H_GAP = 12
/** Vertical gap between rows of submodels. */
const V_GAP = 12
/** Padding inside a group node (sides and bottom). */
const PADDING = 16
/** Maximum number of submodel columns in a group node. */
const MAX_SUBMODEL_COLS = 2
/**
 * Fallback header height when no measurement is available yet.
 * Deliberately generous so the initial render is more likely to be too big
 * than too small, reducing the visual jump on the first measurement pass.
 */
const HEADER_H_FALLBACK = 64
/**
 * Fallback per-parameter row height before measurement.
 * Also generous for the same reason.
 */
const PARAM_ROW_H_FALLBACK = 36
/** Minimum node width — content is rendered at this width during measurement. */
const LEAF_MIN_W = 340

// ── Size computation (bottom-up) ──────────────────────────────────────────────

interface Size {
    width: number
    height: number
}

/**
 * Computes the pixel size required to render `node` and all its descendants
 * as nested boxes.  `contentSizes` supplies the accurately-measured content
 * dimensions of each node's own card (header + parameters); when absent for a
 * node the fallback constants above are used.  Must be called before `placeNodes`.
 */
function computeSize(
    node: RenderedNode,
    cache: Map<string, Size>,
    contentSizes: Map<string, ContentSize> | null,
): Size {
    const id = instanceId(node)
    const cached = cache.get(id)
    if (cached) return cached

    for (const child of node.children) computeSize(child.node, cache, contentSizes)

    const measured = contentSizes?.get(id)
    const contentH =
        measured?.height ??
        HEADER_H_FALLBACK + node.parameters.length * PARAM_ROW_H_FALLBACK
    const contentW = measured?.width ?? LEAF_MIN_W

    let size: Size
    if (node.children.length === 0) {
        size = { width: contentW, height: contentH + PADDING }
    } else {
        // Arrange children in a grid with MAX_SUBMODEL_COLS columns
        const childSizes = node.children.map((c) => cache.get(instanceId(c.node))!)
        const numCols = Math.min(childSizes.length, MAX_SUBMODEL_COLS)
        const numRows = Math.ceil(childSizes.length / MAX_SUBMODEL_COLS)

        // Calculate column widths (max width in each column)
        const colWidths: number[] = []
        for (let col = 0; col < numCols; col++) {
            let maxW = 0
            for (let row = 0; row < numRows; row++) {
                const idx = row * MAX_SUBMODEL_COLS + col
                if (idx < childSizes.length) {
                    maxW = Math.max(maxW, childSizes[idx].width)
                }
            }
            colWidths.push(maxW)
        }

        // Calculate row heights (max height in each row)
        const rowHeights: number[] = []
        for (let row = 0; row < numRows; row++) {
            let maxH = 0
            for (let col = 0; col < numCols; col++) {
                const idx = row * MAX_SUBMODEL_COLS + col
                if (idx < childSizes.length) {
                    maxH = Math.max(maxH, childSizes[idx].height)
                }
            }
            rowHeights.push(maxH)
        }

        const gridW = colWidths.reduce((s, w) => s + w, 0) + (numCols - 1) * H_GAP
        const gridH = rowHeights.reduce((s, h) => s + h, 0) + (numRows - 1) * V_GAP

        size = {
            width: Math.max(contentW, gridW + PADDING * 2),
            height: contentH + PADDING + gridH + PADDING,
        }
    }

    cache.set(id, size)
    return size
}

// ── Node placement ────────────────────────────────────────────────────────────

type FlowNode = Node<ModelNodeData>
interface ModelNodeData {
    node: RenderedNode
    alias: string | null
    /** "main" for main tree nodes, or { modelPath } for reference pool nodes */
    treeType: "main" | { modelPath: string }
}

/**
 * Recursively builds the flat reactflow node list.
 * Parent nodes must be pushed before their children (reactflow requirement).
 * Child positions are relative to their parent's top-left corner.
 *
 * @param idPrefix - Prefix for node IDs (used for reference pool nodes).
 * @param treeType - "main" for main tree, or { modelPath } for reference pool.
 */
function placeNodes(
    node: RenderedNode,
    alias: string | null,
    parentId: string | null,
    x: number,
    y: number,
    sizes: Map<string, Size>,
    contentSizes: Map<string, ContentSize> | null,
    idPrefix: string,
    treeType: "main" | { modelPath: string },
    out: FlowNode[],
): void {
    const baseId = instanceId(node)
    const id = idPrefix + baseId
    const size = sizes.get(baseId)!
    const isGroup = node.children.length > 0

    const flowNode: FlowNode = {
        id,
        position: { x, y },
        data: { node, alias, treeType },
        style: { width: size.width, height: size.height },
        type: isGroup ? "groupModel" : "leafModel",
        ...(parentId !== null ? { parentNode: parentId, extent: "parent" as const } : {}),
    }
    out.push(flowNode)

    if (isGroup) {
        const measuredContentH = contentSizes?.get(baseId)?.height
        const contentH =
            measuredContentH ??
            HEADER_H_FALLBACK + node.parameters.length * PARAM_ROW_H_FALLBACK

        // Arrange children in a grid with MAX_SUBMODEL_COLS columns
        const childSizes = node.children.map((c) => sizes.get(instanceId(c.node))!)
        const numCols = Math.min(childSizes.length, MAX_SUBMODEL_COLS)
        const numRows = Math.ceil(childSizes.length / MAX_SUBMODEL_COLS)

        // Calculate column widths and row heights for positioning
        const colWidths: number[] = []
        for (let col = 0; col < numCols; col++) {
            let maxW = 0
            for (let row = 0; row < numRows; row++) {
                const idx = row * MAX_SUBMODEL_COLS + col
                if (idx < childSizes.length) {
                    maxW = Math.max(maxW, childSizes[idx].width)
                }
            }
            colWidths.push(maxW)
        }
        const rowHeights: number[] = []
        for (let row = 0; row < numRows; row++) {
            let maxH = 0
            for (let col = 0; col < numCols; col++) {
                const idx = row * MAX_SUBMODEL_COLS + col
                if (idx < childSizes.length) {
                    maxH = Math.max(maxH, childSizes[idx].height)
                }
            }
            rowHeights.push(maxH)
        }

        // Place children in grid positions
        let currentY = contentH + PADDING
        for (let row = 0; row < numRows; row++) {
            let currentX = PADDING
            for (let col = 0; col < numCols; col++) {
                const idx = row * MAX_SUBMODEL_COLS + col
                if (idx < node.children.length) {
                    const child = node.children[idx]
                    placeNodes(child.node, child.alias, id, currentX, currentY, sizes, contentSizes, idPrefix, treeType, out)
                }
                currentX += colWidths[col] + H_GAP
            }
            currentY += rowHeights[row] + V_GAP
        }
    }
}

/**
 * Builds the complete flat reactflow node list from the instance tree.
 * `contentSizes` is `null` on the very first render (before any measurements
 * have arrived) and the fallback constants are used instead.
 *
 * @param idPrefix - Optional prefix for node IDs (used for reference pool nodes).
 * @param treeType - "main" for main tree, or { modelPath } for reference pool.
 */
function buildElements(
    root: RenderedNode,
    contentSizes: Map<string, ContentSize> | null,
    idPrefix = "",
    treeType: "main" | { modelPath: string } = "main",
): FlowNode[] {
    const sizes = new Map<string, Size>()
    computeSize(root, sizes, contentSizes)
    const nodes: FlowNode[] = []
    placeNodes(root, null, null, 0, 0, sizes, contentSizes, idPrefix, treeType, nodes)
    return nodes
}

/** Stable node ID derived from the instance path. */
function instanceId(node: RenderedNode): string {
    return node.instance_path.length > 0 ? node.instance_path.join("/") : "__root__"
}

/** Returns every node in the tree (root-first, depth-first). */
function flattenNodes(node: RenderedNode): RenderedNode[] {
    return [node, ...node.children.flatMap((c) => flattenNodes(c.node))]
}

/** Builds a `design_name → color_index` map from a node's applied_designs. */
function buildDesignIndex(designs: AppliedDesign[]): Map<string, number> {
    return new Map(designs.map((d) => [d.design_name, d.color_index]))
}

// ── Pure (atom-free) content components ──────────────────────────────────────
// These take all display options as props so they can be used both in the
// visible graph nodes and in the hidden measurement container without needing
// a Jotai store in scope.

/** Renders the header section of a model card (name + design badges + note). */
function NodeHeader({
    node,
    alias,
    showNotes,
    enableTooltip = false,
}: {
    node: RenderedNode
    alias: string | null
    showNotes: boolean
    enableTooltip?: boolean
}) {
    // Always show the model type name, not the alias
    const name = modelDisplayName(node.model_path)
    const tooltipProps = useTooltipTrigger(enableTooltip && !showNotes ? node.note : undefined)
    return (
        <div className="graph-node-header">
            {/* Alias tab: overlaps top border to show connection from parent */}
            {alias && <div className="graph-node-alias-tab">{alias}</div>}
            <div className="graph-node-header-top">
                <div
                    className={`graph-node-name${tooltipProps.className ? ` ${tooltipProps.className}` : ""}`}
                    onMouseEnter={tooltipProps.onMouseEnter}
                    onMouseLeave={tooltipProps.onMouseLeave}
                >
                    {name}
                </div>
                <div className="graph-node-badges">
                    {node.applied_designs.map((d) => (
                        <span
                            key={d.design_name}
                            className="design-badge"
                            style={{
                                color: designColorVar(d.color_index),
                                borderColor: designColorVar(d.color_index),
                            }}
                        >
                            {d.design_name}
                        </span>
                    ))}
                </div>
            </div>
            {showNotes && node.note && (
                <div className="graph-node-note">
                    <NoteDisplay text={node.note} />
                </div>
            )}
        </div>
    )
}

/** Renders the parameter list for a model card. Pure — all display options are props. */
function ParamListContent({
    params,
    designIndex,
    showDesigns,
    showNotes,
    showTrace,
    instancePath,
    treeType = "main",
    enableTooltip = false,
}: {
    params: RenderedParameter[]
    designIndex: Map<string, number>
    showDesigns: boolean
    showNotes: boolean
    showTrace: boolean
    instancePath: string[]
    treeType?: "main" | { modelPath: string }
    enableTooltip?: boolean
}) {
    const visibleParams = showTrace
        ? params
        : params.filter((p) => p.print_level !== "trace")
    if (visibleParams.length === 0) return null
    return (
        <div className="graph-node-params">
            {visibleParams.map((p) => (
                <ParamRow
                    key={p.name}
                    param={p}
                    designIndex={designIndex}
                    showDesigns={showDesigns}
                    showNotes={showNotes}
                    instancePath={instancePath}
                    treeType={treeType}
                    enableTooltip={enableTooltip}
                />
            ))}
        </div>
    )
}

function ParamRow({
    param: p,
    designIndex,
    showDesigns,
    showNotes,
    instancePath,
    treeType,
    enableTooltip,
}: {
    param: RenderedParameter
    designIndex: Map<string, number>
    showDesigns: boolean
    showNotes: boolean
    instancePath: string[]
    treeType: "main" | { modelPath: string }
    enableTooltip: boolean
}) {
    const [highlightedDeps, setHighlightedDeps] = useAtom(highlightedDepsAtom)
    const { aliasToModelPath } = useContext(GraphDepContext)
    const tooltipProps = useTooltipTrigger(enableTooltip && !showNotes ? p.note : undefined)
    const mark = p.design
    const colorIdx = mark != null ? (designIndex.get(mark.design_name) ?? 0) : null

    // Extract dependency keys for hover highlighting
    const depKeys = useMemo(
        () => extractDependencyKeys(p.expression as ParameterValueAst | null, instancePath, aliasToModelPath),
        [p.expression, instancePath, aliasToModelPath],
    )

    // Compute this parameter's key for highlight checking
    const myKey = useMemo(() => {
        if (treeType === "main") {
            return mainTreeParamKey(instancePath, p.name)
        }
        return refPoolParamKey(treeType.modelPath, p.name)
    }, [treeType, instancePath, p.name])

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

    const showExpr =
        p.expression != null && !isSimpleLiteral(p.expression as ParameterValueAst)
    const isPerformance = p.print_level === "performance"

    // Apply design styling to label (since display:contents removes row box)
    const labelStyle: React.CSSProperties =
        showDesigns && mark != null && colorIdx != null
            ? {
                  borderLeftColor: designColorVar(colorIdx),
                  borderLeftWidth: "var(--design-border-width)",
                  borderLeftStyle: "solid",
                  paddingLeft: "4px",
                  ...(mark.is_addition
                      ? {
                            backgroundColor: `color-mix(in srgb, ${designColorVar(colorIdx)} var(--design-color-tint-alpha), transparent)`,
                        }
                      : {}),
              }
            : {}

    const rowClasses = [
        "graph-node-param-row",
        isPerformance && "param-performance",
        isHighlighted && "param-highlighted",
    ]
        .filter(Boolean)
        .join(" ")

    return (
        <div className={rowClasses}>
            <span
                className={`graph-param-label${tooltipProps.className ? ` ${tooltipProps.className}` : ""}`}
                style={labelStyle}
                onMouseEnter={tooltipProps.onMouseEnter}
                onMouseLeave={tooltipProps.onMouseLeave}
            >
                {p.label}
            </span>
            {showExpr ? (
                <span className="graph-param-expr">
                    <GraphExprDisplay expr={p.expression as ParameterValueAst} />
                </span>
            ) : (
                <span className="graph-param-expr" />
            )}
            <span className="graph-param-sep">:</span>
            <span className="graph-param-value">
                <span
                    className={`param-name${depKeys.size > 0 ? " has-deps" : ""}`}
                    onMouseEnter={onNameMouseEnter}
                    onMouseLeave={onNameMouseLeave}
                >
                    {p.name}
                </span>
                {" = "}
                <ValueDisplay value={p.value} />
            </span>
            {showNotes && p.note && (
                <span className="graph-param-note">
                    <NoteDisplay text={p.note} />
                </span>
            )}
        </div>
    )
}

/** Reads atoms and delegates to `ParamListContent`. */
function ParamList({
    params,
    designIndex,
    instancePath,
    treeType,
    enableTooltip = false,
}: {
    params: RenderedParameter[]
    designIndex: Map<string, number>
    instancePath: string[]
    treeType: "main" | { modelPath: string }
    enableTooltip?: boolean
}) {
    const showDesigns = useAtomValue(showDesignsAtom)
    const showNotes = useAtomValue(showNotesAtom)
    const showTrace = useAtomValue(showTraceAtom)
    return (
        <ParamListContent
            params={params}
            designIndex={designIndex}
            showDesigns={showDesigns}
            showNotes={showNotes}
            showTrace={showTrace}
            instancePath={instancePath}
            treeType={treeType}
            enableTooltip={enableTooltip}
        />
    )
}

function GraphExprDisplay({ expr }: { expr: ParameterValueAst }) {
    try {
        const html = katex.renderToString(paramExprOnlyToLatex(expr), {
            output: "mathml",
            throwOnError: false,
        })
        return <span dangerouslySetInnerHTML={{ __html: html }} />
    } catch {
        return null
    }
}

// ── Measurement-only content component ───────────────────────────────────────

/**
 * Renders just the card content (header + params) for a node.  Used exclusively
 * in the hidden measurement pass — no Jotai atoms, and design overlay styling is
 * omitted because it does not affect height.
 */
function MeasureNodeContent({
    node,
    showNotes,
    showTrace,
}: {
    node: RenderedNode
    showNotes: boolean
    showTrace: boolean
}) {
    const designIndex = useMemo(
        () => buildDesignIndex(node.applied_designs),
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [node],
    )
    return (
        <div className="graph-node-card">
            <NodeHeader node={node} alias={null} showNotes={showNotes} />
            <ParamListContent
                params={node.parameters}
                designIndex={designIndex}
                showDesigns={false}
                showNotes={showNotes}
                showTrace={showTrace}
                instancePath={node.instance_path}
            />
        </div>
    )
}

// ── Custom reactflow node renderers ───────────────────────────────────────────

/** Node with no children — standalone card. */
function LeafModelNode({ data }: { data: ModelNodeData }) {
    const showNotes = useAtomValue(showNotesAtom)
    const designIndex = useMemo(
        () => buildDesignIndex(data.node.applied_designs),
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [data.node],
    )
    return (
        <div className="graph-node-card graph-node-leaf">
            <Handle type="target" position={Position.Top} style={{ opacity: 0 }} />
            <NodeHeader node={data.node} alias={data.alias} showNotes={showNotes} enableTooltip />
            <ParamList
                params={data.node.parameters}
                designIndex={designIndex}
                instancePath={data.node.instance_path}
                treeType={data.treeType}
                enableTooltip
            />
            <Handle type="source" position={Position.Bottom} style={{ opacity: 0 }} />
        </div>
    )
}

/** Node that contains children — renders only its own header/params; children
 *  are separate reactflow nodes positioned inside this node's bounds. */
function GroupModelNode({ data }: { data: ModelNodeData }) {
    const showNotes = useAtomValue(showNotesAtom)
    const designIndex = useMemo(
        () => buildDesignIndex(data.node.applied_designs),
        // eslint-disable-next-line react-hooks/exhaustive-deps
        [data.node],
    )
    return (
        <div className="graph-node-card graph-node-group">
            <NodeHeader node={data.node} alias={data.alias} showNotes={showNotes} enableTooltip />
            <ParamList
                params={data.node.parameters}
                designIndex={designIndex}
                instancePath={data.node.instance_path}
                treeType={data.treeType}
                enableTooltip
            />
        </div>
    )
}

const nodeTypes = { leafModel: LeafModelNode, groupModel: GroupModelNode }

// ── Public component ──────────────────────────────────────────────────────────

interface ModelGraphViewProps {
    node: RenderedNode
    referencePool: RenderedPoolEntry[]
}

/**
 * Renders the instance tree as nested boxes using reactflow's parent-node
 * feature. Also renders the reference pool as separate top-level nodes.
 *
 * **Layout sizing** uses a two-phase approach:
 * 1. On every render a hidden off-screen pass renders each node's content at
 *    `LEAF_MIN_W` pixels wide and measures the natural height with
 *    `ResizeObserver` via `useMeasureContent`.
 * 2. `computeSize` uses those measured heights (falling back to generous
 *    constant estimates until the first measurement arrives) to build the
 *    reactflow node sizes.
 *
 * Because the hidden measurement container lives *inside* the component tree
 * (not in a portal), it inherits the same CSS context — including the
 * user-controlled font scale applied to `.app` — and the `ResizeObserver`
 * fires automatically whenever the font size changes, keeping the layout
 * accurate without any extra wiring.
 */
export function ModelGraphView({ node, referencePool }: ModelGraphViewProps) {
    const showNotes = useAtomValue(showNotesAtom)
    const showTrace = useAtomValue(showTraceAtom)

    // Build alias → model_path mapping once for the whole tree
    const aliasToModelPath = useMemo(() => buildAliasToModelPath(node), [node])

    // Main tree context
    const mainCtx: GraphDepContext = useMemo(
        () => ({ aliasToModelPath, treeType: "main" }),
        [aliasToModelPath],
    )

    // Collect all nodes (main tree + reference pool) for measurement.
    const measureItems: MeasureItem[] = useMemo(() => {
        const mainNodes = flattenNodes(node).map((n) => ({
            id: instanceId(n),
            element: <MeasureNodeContent node={n} showNotes={showNotes} showTrace={showTrace} />,
        }))
        const refNodes = referencePool.flatMap((entry) =>
            flattenNodes(entry.node).map((n) => ({
                id: `ref:${entry.alias}:${instanceId(n)}`,
                element: <MeasureNodeContent node={n} showNotes={showNotes} showTrace={showTrace} />,
            })),
        )
        return [...mainNodes, ...refNodes]
    }, [node, referencePool, showNotes, showTrace])

    const { sizes: contentSizes, container: measureContainer } = useMeasureContent(
        measureItems,
        LEAF_MIN_W,
    )

    // Rebuild the reactflow node list for the main tree.
    const mainNodes = useMemo(() => buildElements(node, contentSizes), [node, contentSizes])

    // Build nodes for the reference pool (to the right of main tree).
    const refNodes = useMemo(() => {
        // Find the total width of the main tree from the root node's computed size
        // (which includes all nested submodels)
        const rootNode = mainNodes.find((n) => n.parentId == null)
        const mainTreeWidth =
            (rootNode?.style?.width as number | undefined) ?? LEAF_MIN_W
        const refStartX = mainTreeWidth + PADDING * 3 // Gap before reference pool

        // Build each reference pool entry, stacked vertically to the right
        const allRefNodes: FlowNode[] = []
        let currentY = 0
        for (const entry of referencePool) {
            const refContentSizes = new Map<string, ContentSize>()
            if (contentSizes) {
                for (const n of flattenNodes(entry.node)) {
                    const key = `ref:${entry.alias}:${instanceId(n)}`
                    const size = contentSizes.get(key)
                    if (size) refContentSizes.set(instanceId(n), size)
                }
            }
            const entryNodes = buildElements(
                entry.node,
                refContentSizes,
                `ref:${entry.alias}:`,
                { modelPath: entry.node.model_path },
            )
            // Position root node at refStartX, stacking vertically
            const entryRoot = entryNodes.find((n) => n.parentId == null)
            if (entryRoot) {
                entryRoot.position = { x: refStartX, y: currentY }
                const entryHeight = (entryRoot.style?.height as number | undefined) ?? 0
                currentY += entryHeight + PADDING * 2
            }
            allRefNodes.push(...entryNodes)
        }
        return allRefNodes
    }, [mainNodes, referencePool, contentSizes])

    const nodes = useMemo(() => [...mainNodes, ...refNodes], [mainNodes, refNodes])

    // Track viewport zoom for tooltip scaling
    const setGraphZoom = useSetAtom(graphZoomAtom)
    const onMoveEnd = useCallback(
        (_event: MouseEvent | TouchEvent | null, viewport: Viewport) => {
            setGraphZoom(viewport.zoom)
        },
        [setGraphZoom],
    )
    // Capture initial zoom after fitView
    const onInit = useCallback(
        (instance: { getViewport: () => Viewport }) => {
            // Small delay to let fitView complete
            setTimeout(() => setGraphZoom(instance.getViewport().zoom), 50)
        },
        [setGraphZoom],
    )

    return (
        // position: relative is required so the absolutely-positioned hidden
        // measurement container is clipped to this element's visual bounds.
        <GraphDepContext.Provider value={mainCtx}>
            <div className="graph-container" style={{ position: "relative" }}>
                {measureContainer}
                <ReactFlow
                    nodes={nodes}
                    edges={[]}
                    nodeTypes={nodeTypes}
                    fitView
                    nodesDraggable={false}
                    nodesConnectable={false}
                    elementsSelectable={false}
                    proOptions={{ hideAttribution: true }}
                    onInit={onInit}
                    onMoveEnd={onMoveEnd}
            >
                <Background variant={BackgroundVariant.Dots} />
                <Controls showInteractive={false} />
            </ReactFlow>
            </div>
        </GraphDepContext.Provider>
    )
}
