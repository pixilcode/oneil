/**
 * Extracts variable dependencies from an expression AST and provides
 * utilities for creating consistent parameter keys.
 *
 * Key format:
 * - Main tree params: `instance_path.join(".") + "." + param.name` (or just param.name at root)
 * - Reference pool params: `model_path + "." + param.name`
 *
 * For dependencies extracted from expressions:
 * - `Variable::Parameter` → same instance_path as the containing param
 * - `Variable::External` → resolved via alias → model_path mapping
 */

import type { ExprAst, ParameterValueAst, PiecewiseExprAst, RenderedNode } from "../types/model"

/**
 * Creates a unique key for a parameter in the main tree.
 * Uses instance_path to create an absolute path from the root.
 */
export function mainTreeParamKey(instancePath: string[], paramName: string): string {
    if (instancePath.length === 0) {
        return paramName
    }
    return `${instancePath.join(".")}.${paramName}`
}

/**
 * Creates a unique key for a parameter in the reference pool.
 * Uses model_path as the root identifier.
 */
export function refPoolParamKey(modelPath: string, paramName: string): string {
    return `ref:${modelPath}.${paramName}`
}

/**
 * Builds a mapping from reference alias → model_path by walking the tree.
 */
export function buildAliasToModelPath(node: RenderedNode): Map<string, string> {
    const map = new Map<string, string>()

    function walk(n: RenderedNode) {
        for (const ref of n.references) {
            map.set(ref.alias, ref.model_path)
        }
        for (const child of n.children) {
            walk(child.node)
        }
    }

    walk(node)
    return map
}

/**
 * Extracts all dependency keys from a parameter's expression.
 *
 * @param expr - The parameter's expression AST
 * @param instancePath - The instance_path of the node containing this parameter
 * @param aliasToModelPath - Mapping from cross-file reference alias to model_path
 *        (does NOT include submodel aliases)
 * @returns Set of parameter keys that this expression depends on
 */
export function extractDependencyKeys(
    expr: ParameterValueAst | null,
    instancePath: string[],
    aliasToModelPath: Map<string, string>,
): Set<string> {
    const keys = new Set<string>()
    if (!expr) return keys

    function walkExpr(e: ExprAst): void {
        if ("Variable" in e) {
            const v = e.Variable.variable
            if ("Parameter" in v) {
                // Local parameter - same instance_path
                keys.add(mainTreeParamKey(instancePath, v.Parameter.parameter_name))
            } else if ("External" in v) {
                // External reference - could be submodel or cross-file reference
                const { reference_name, parameter_name } = v.External
                const modelPath = aliasToModelPath.get(reference_name)
                if (modelPath) {
                    // Cross-file reference (in reference pool)
                    keys.add(refPoolParamKey(modelPath, parameter_name))
                } else {
                    // Submodel reference (in main tree) - extend instance path
                    keys.add(mainTreeParamKey([...instancePath, reference_name], parameter_name))
                }
            }
            // Builtins are ignored
        } else if ("BinaryOp" in e) {
            walkExpr(e.BinaryOp.left)
            walkExpr(e.BinaryOp.right)
        } else if ("UnaryOp" in e) {
            walkExpr(e.UnaryOp.expr)
        } else if ("ComparisonOp" in e) {
            walkExpr(e.ComparisonOp.left)
            walkExpr(e.ComparisonOp.right)
            for (const [, rest] of e.ComparisonOp.rest_chained) {
                walkExpr(rest)
            }
        } else if ("FunctionCall" in e) {
            for (const arg of e.FunctionCall.args) {
                walkExpr(arg)
            }
        } else if ("UnitCast" in e) {
            walkExpr(e.UnitCast.expr)
        }
        // Literals have no dependencies
    }

    function walkPiecewise(pw: PiecewiseExprAst): void {
        walkExpr(pw.expr)
        walkExpr(pw.if_expr)
    }

    if ("Simple" in expr) {
        walkExpr(expr.Simple[0])
    } else if ("Piecewise" in expr) {
        for (const pw of expr.Piecewise[0]) {
            walkPiecewise(pw)
        }
    }

    return keys
}
