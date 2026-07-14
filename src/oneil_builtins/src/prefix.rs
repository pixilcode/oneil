//! Standard builtin unit prefixes (e.g. `k`, `m`, `M`).

use oneil_shared::symbols::UnitPrefix;

/// A builtin unit prefix (e.g. "k" for kilo, "m" for milli).
#[derive(Debug, Clone)]
pub struct BuiltinPrefix {
    pub prefix: UnitPrefix,
    pub value: f64,
    pub description: &'static str,
}

/// Returns an iterator over all standard builtin prefixes.
#[expect(clippy::too_many_lines, reason = "this is a list of builtin prefixes")]
pub fn builtin_prefixes_complete() -> impl Iterator<Item = (UnitPrefix, BuiltinPrefix)> {
    [
        BuiltinPrefix {
            prefix: UnitPrefix::from("q"),
            value: 1e-30,
            description: "quecto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("r"),
            value: 1e-27,
            description: "ronto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("y"),
            value: 1e-24,
            description: "yocto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("z"),
            value: 1e-21,
            description: "zepto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("a"),
            value: 1e-18,
            description: "atto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("f"),
            value: 1e-15,
            description: "femto",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("p"),
            value: 1e-12,
            description: "pico",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("n"),
            value: 1e-9,
            description: "nano",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("u"),
            value: 1e-6,
            description: "micro",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("m"),
            value: 1e-3,
            description: "milli",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("k"),
            value: 1e3,
            description: "kilo",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("M"),
            value: 1e6,
            description: "mega",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("G"),
            value: 1e9,
            description: "giga",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("T"),
            value: 1e12,
            description: "tera",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("P"),
            value: 1e15,
            description: "peta",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("E"),
            value: 1e18,
            description: "exa",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("Z"),
            value: 1e21,
            description: "zetta",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("Y"),
            value: 1e24,
            description: "yotta",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("R"),
            value: 1e27,
            description: "ronna",
        },
        BuiltinPrefix {
            prefix: UnitPrefix::from("Q"),
            value: 1e30,
            description: "quetta",
        },
    ]
    .into_iter()
    .map(|prefix| (prefix.prefix.clone(), prefix))
}
