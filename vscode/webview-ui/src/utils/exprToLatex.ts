/**
 * Converts a serialized `ir::ParameterValue` AST into a LaTeX string for
 * KaTeX rendering.
 *
 * All types are driven from the actual Rust serde output — see comments in
 * `types/model.ts` for the key serialization rules.
 */

import type {
    BinaryOpAst,
    ComparisonOpAst,
    ExprAst,
    FunctionNameAst,
    LiteralAst,
    ParameterValueAst,
    PiecewiseExprAst,
    UnaryOpAst,
    VariableAst,
} from "../types/model"

// ── Precedence levels (higher binds tighter) ─────────────────────────────────

const PREC = {
    addSub:  2,
    mul:     3,
    unary:   5,
    pow:     6,
    atom:    7,
} as const

type Prec = number

// ── Public entry point ────────────────────────────────────────────────────────

/**
 * Converts a `ParameterValueAst` to a LaTeX string.
 *
 * For `Simple` values renders `expr\,\mathrm{unit}` (unit optional).
 * For `Piecewise` values renders a `\begin{cases}…\end{cases}` block.
 *
 * @example
 * ```ts
 * paramValueToLatex({ Simple: [{ Literal: { span: 0, value: { Number: 9.81 } } }, "m/s^2"] })
 * // → "9.81\\,\\mathrm{m/s^2}"
 * ```
 */
export function paramValueToLatex(ast: ParameterValueAst): string {
    if ("Simple" in ast) {
        const [expr, unit] = ast.Simple
        // Pass 0 as the parent precedence so the top-level expression is never
        // wrapped in extra parentheses — surrounding context requires none.
        const exprLatex = exprToLatex(expr, 0)
        return unit != null ? `${exprLatex}\\,\\mathrm{${escapeUnit(unit)}}` : exprLatex
    }

    const [pieces, unit] = ast.Piecewise
    const body = piecewiseToLatex(pieces)
    return unit != null ? `${body}\\,\\mathrm{${escapeUnit(unit)}}` : body
}

/**
 * Like `paramValueToLatex` but **omits the unit suffix** from the rendered
 * expression.  Use this when the numeric value (which already carries its
 * unit string) is displayed right next to the equation — there is no need to
 * repeat the unit inside the LaTeX.
 */
export function paramExprOnlyToLatex(ast: ParameterValueAst): string {
    if ("Simple" in ast) {
        const [expr] = ast.Simple
        return exprToLatex(expr, 0)
    }
    const [pieces] = ast.Piecewise
    return piecewiseToLatex(pieces)
}

/**
 * Returns `true` when the expression is a bare literal — rendering it as
 * an equation would add no information over the already-displayed value.
 *
 * @example
 * ```ts
 * isSimpleLiteral({ Simple: [{ Literal: { span: 0, value: { Number: 9.81 } } }, null] })
 * // → true
 * ```
 */
export function isSimpleLiteral(ast: ParameterValueAst): boolean {
    if ("Piecewise" in ast) return false
    const [expr] = ast.Simple
    return "Literal" in expr
}

// ── Piecewise ─────────────────────────────────────────────────────────────────

function piecewiseToLatex(pieces: PiecewiseExprAst[]): string {
    const rows = pieces
        .map((p) => `${exprToLatex(p.expr, PREC.atom)} & \\text{if } ${exprToLatex(p.if_expr, PREC.atom)}`)
        .join(" \\\\ ")
    return `\\begin{cases} ${rows} \\end{cases}`
}

// ── Expression ────────────────────────────────────────────────────────────────

function exprToLatex(expr: ExprAst, parentPrec: Prec): string {
    if ("Literal" in expr) {
        return literalToLatex(expr.Literal.value)
    }

    if ("Variable" in expr) {
        return variableToLatex(expr.Variable.variable)
    }

    if ("BinaryOp" in expr) {
        const { op, left, right } = expr.BinaryOp
        return binaryOpToLatex(op, left, right, parentPrec)
    }

    if ("UnaryOp" in expr) {
        const { op, expr: operand } = expr.UnaryOp
        return unaryOpToLatex(op, operand, parentPrec)
    }

    if ("ComparisonOp" in expr) {
        const { op, left, right, rest_chained } = expr.ComparisonOp
        // Build chain: a < b < c
        let latex = `${exprToLatex(left, PREC.atom)} ${comparisonSym(op)} ${exprToLatex(right, PREC.atom)}`
        for (const [chainOp, chainExpr] of rest_chained) {
            latex += ` ${comparisonSym(chainOp)} ${exprToLatex(chainExpr, PREC.atom)}`
        }
        return maybeParen(latex, 1, parentPrec)
    }

    if ("FunctionCall" in expr) {
        const { name, args } = expr.FunctionCall
        return functionCallToLatex(name, args)
    }

    if ("UnitCast" in expr) {
        // Render as: expr\,[\mathrm{unit}]
        const inner = exprToLatex(expr.UnitCast.expr, PREC.atom)
        return `${inner}\\,\\left[\\mathrm{${escapeUnit(expr.UnitCast.unit)}}\\right]`
    }

    return "?"
}

// ── Literals ──────────────────────────────────────────────────────────────────

function literalToLatex(lit: LiteralAst): string {
    if ("Number" in lit) return formatNumber(lit.Number)
    if ("String" in lit) return `\\text{${escapeText(lit.String)}}`
    return `\\text{${lit.Boolean}}`
}

// ── Binary operators ──────────────────────────────────────────────────────────

function binaryOpToLatex(op: BinaryOpAst, left: ExprAst, right: ExprAst, parentPrec: Prec): string {
    switch (op) {
        case "add": {
            const inner = `${exprToLatex(left, PREC.addSub)} + ${exprToLatex(right, PREC.addSub)}`
            return maybeParen(inner, PREC.addSub, parentPrec)
        }
        case "sub":
        case "escaped_sub": {
            const inner = `${exprToLatex(left, PREC.addSub)} - ${exprToLatex(right, PREC.addSub + 1)}`
            return maybeParen(inner, PREC.addSub, parentPrec)
        }
        case "mul": {
            const inner = `${exprToLatex(left, PREC.mul)} \\cdot ${exprToLatex(right, PREC.mul)}`
            return maybeParen(inner, PREC.mul, parentPrec)
        }
        case "div":
        case "escaped_div": {
            return `\\frac{${exprToLatex(left, PREC.atom)}}{${exprToLatex(right, PREC.atom)}}`
        }
        case "mod": {
            const inner = `${exprToLatex(left, PREC.mul)} \\bmod ${exprToLatex(right, PREC.mul)}`
            return maybeParen(inner, PREC.mul, parentPrec)
        }
        case "pow": {
            return `{${exprToLatex(left, PREC.pow)}}^{${exprToLatex(right, PREC.atom)}}`
        }
        case "and": {
            const inner = `${exprToLatex(left, PREC.addSub)} \\land ${exprToLatex(right, PREC.addSub)}`
            return maybeParen(inner, PREC.addSub, parentPrec)
        }
        case "or": {
            const inner = `${exprToLatex(left, PREC.addSub)} \\lor ${exprToLatex(right, PREC.addSub)}`
            return maybeParen(inner, PREC.addSub, parentPrec)
        }
        case "min_max": {
            // a | b → \min(a, b) or \max(a, b) — we don't know which so use the source notation
            return `\\left(${exprToLatex(left, PREC.atom)} \\mid ${exprToLatex(right, PREC.atom)}\\right)`
        }
    }
}

// ── Unary operators ───────────────────────────────────────────────────────────

function unaryOpToLatex(op: UnaryOpAst, operand: ExprAst, parentPrec: Prec): string {
    switch (op) {
        case "neg": {
            const inner = `-${exprToLatex(operand, PREC.unary)}`
            return maybeParen(inner, PREC.unary, parentPrec)
        }
        case "not": {
            const inner = `\\lnot ${exprToLatex(operand, PREC.unary)}`
            return maybeParen(inner, PREC.unary, parentPrec)
        }
    }
}

// ── Variables ─────────────────────────────────────────────────────────────────

function variableToLatex(v: VariableAst): string {
    if ("Parameter" in v) return mathName(v.Parameter.parameter_name)
    if ("Builtin" in v)   return mathName(v.Builtin.ident)
    // External: g.planet → g_{planet}  (Oneil source order: param.ref)
    return `${mathName(v.External.parameter_name)}_{${mathName(v.External.reference_name)}}`
}

// ── Function calls ────────────────────────────────────────────────────────────

function functionCallToLatex(name: FunctionNameAst, args: ExprAst[]): string {
    const argsLatex = args.map((a) => exprToLatex(a, PREC.atom))

    if ("Builtin" in name) {
        const builtinName = name.Builtin[0]

        // Special-case functions that have non-standard LaTeX syntax
        switch (builtinName) {
            case "sqrt":
                return `\\sqrt{${argsLatex.join(", ")}}`
            case "abs":
                return `\\left|${argsLatex.join(", ")}\\right|`
            case "floor":
                return `\\left\\lfloor ${argsLatex.join(", ")} \\right\\rfloor`
            case "ceil":
                return `\\left\\lceil ${argsLatex.join(", ")} \\right\\rceil`
        }

        // Standard operator-style functions: \sin, \cos, etc.
        const operatorMap: Record<string, string> = {
            sin:  "\\sin",
            cos:  "\\cos",
            tan:  "\\tan",
            asin: "\\arcsin",
            acos: "\\arccos",
            atan: "\\arctan",
            ln:   "\\ln",
            log:  "\\log",
            exp:  "\\exp",
            min:  "\\min",
            max:  "\\max",
        }
        const op = operatorMap[builtinName]
        if (op) {
            return `${op}\\left(${argsLatex.join(", ")}\\right)`
        }

        // Unknown builtin — render as upright text
        return `\\mathrm{${escapeIdent(builtinName)}}\\left(${argsLatex.join(", ")}\\right)`
    }

    // Imported Python function
    return `\\mathrm{${escapeIdent(name.Imported.name)}}\\left(${argsLatex.join(", ")}\\right)`
}

// ── Comparison operators ──────────────────────────────────────────────────────

function comparisonSym(op: ComparisonOpAst): string {
    switch (op) {
        case "eq":             return "="
        case "not_eq":         return "\\neq"
        case "less_than":      return "<"
        case "less_than_eq":   return "\\leq"
        case "greater_than":   return ">"
        case "greater_than_eq": return "\\geq"
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function maybeParen(inner: string, ownPrec: Prec, parentPrec: Prec): string {
    return ownPrec < parentPrec ? `\\left(${inner}\\right)` : inner
}

function formatNumber(n: number): string {
    if (!isFinite(n)) return n > 0 ? "\\infty" : "-\\infty"
    if (Number.isInteger(n) && Math.abs(n) < 1e9) return String(n)
    const abs = Math.abs(n)
    if (abs !== 0 && (abs >= 1e6 || abs < 1e-3)) {
        const exp = Math.floor(Math.log10(abs))
        const mantissa = n / Math.pow(10, exp)
        const mantissaStr = trimTrailingZeros(mantissa.toPrecision(4))
        if (mantissaStr === "1")  return `10^{${exp}}`
        if (mantissaStr === "-1") return `-10^{${exp}}`
        return `${mantissaStr} \\times 10^{${exp}}`
    }
    return trimTrailingZeros(n.toPrecision(6))
}

function trimTrailingZeros(s: string): string {
    return s.includes(".") ? s.replace(/\.?0+$/, "") : s
}

/** Wraps an identifier in `\mathrm{…}` with underscore-separated parts as subscripts. */
function mathName(name: string): string {
    const parts = name.split("_")
    if (parts.length === 1) return `\\mathrm{${escapeIdent(name)}}`
    const [head, ...tail] = parts
    return `\\mathrm{${escapeIdent(head)}}_{\\mathrm{${tail.map(escapeIdent).join("\\,")}}}`
}

function escapeIdent(s: string): string {
    return s.replace(/[#$%&_{}\\^~]/g, (c) => `\\${c}`)
}

function escapeText(s: string): string {
    return s.replace(/[#$%&_{}\\^~]/g, (c) => `\\${c}`)
}

function escapeUnit(unit: string): string {
    return unit.replace(/[#$%&{}\\~]/g, (c) => `\\${c}`)
}
