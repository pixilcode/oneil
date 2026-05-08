/**
 * Returns the display name for a model file path: the last path segment with
 * any `.on` or `.one` extension stripped.
 *
 * @example
 * ```ts
 * modelDisplayName("/path/to/vehicle.on")  // "vehicle"
 * modelDisplayName("/path/to/overlay.one") // "overlay"
 * modelDisplayName("engine")               // "engine"
 * ```
 */
export function modelDisplayName(modelPath: string): string {
    const segment = modelPath.split("/").at(-1) ?? modelPath
    return segment.replace(/\.(on|one)$/, "")
}

/**
 * Utility helpers for the design overlay color system.
 *
 * The palette is defined as `--design-color-N` CSS custom properties in
 * `index.css`. Components read them at render time so they automatically
 * respect any theme overrides.
 */

const PALETTE_SIZE = 8

/**
 * Returns the CSS `var(--design-color-N)` string for a given `color_index`.
 * Wraps around if the index exceeds the palette size.
 *
 * @example
 * ```ts
 * designColorVar(0) // "var(--design-color-0)"
 * designColorVar(9) // "var(--design-color-1)"
 * ```
 */
export function designColorVar(colorIndex: number): string {
    return `var(--design-color-${colorIndex % PALETTE_SIZE})`
}

/**
 * Returns an inline style object that sets `--design-color` to the palette
 * entry for `color_index`. Use this on a wrapper element so child selectors
 * can reference `var(--design-color)` without knowing the index.
 *
 * @example
 * ```tsx
 * <div style={designColorStyle(node.applied_designs[0].color_index)}>
 *   <span style={{ color: "var(--design-color)" }}>badge</span>
 * </div>
 * ```
 */
export function designColorStyle(colorIndex: number): React.CSSProperties {
    return { "--design-color": designColorVar(colorIndex) } as React.CSSProperties
}
