//! Command-line interface definitions for the Oneil CLI

use clap::{Args, Parser, Subcommand};
#[cfg(feature = "python")]
use oneil_shared::paths::PythonPath;
use oneil_shared::{
    paths::ModelPath,
    symbols::{BuiltinFunctionName, BuiltinValueName, ParameterName, UnitBaseName, UnitPrefix},
};
#[cfg(feature = "python")]
use std::path::PathBuf;
use std::{fmt, path::Path, str};

/// Oneil language CLI - Main command-line interface structure
#[derive(Parser)]
#[command(name = "oneil")]
#[command(version, about = "Oneil language tooling", long_about = None)]
pub struct CliCommand {
    /// The subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Number of significant figures to print
    #[arg(long, default_value_t = 4)]
    pub sig_figs: usize,

    /// Disable colors in the output
    ///
    /// When enabled, suppresses colored output for better compatibility with
    /// terminals that don't support ANSI color codes or for redirecting to files.
    #[arg(long)]
    pub no_colors: bool,

    /// Path to the Python virtual environment (venv) to use
    ///
    /// When set, the venv's `bin` (or `Scripts` on Windows) directory is prepended to
    /// `PATH`. If not set and `VIRTUAL_ENV` is unset, the CLI searches upward for a
    /// `venv` or `.venv` directory and uses the first one found.
    #[cfg(feature = "python")]
    #[arg(long, value_name = "VENV")]
    pub venv_path: Option<PathBuf>,

    /// Show internal errors
    #[arg(long, hide = true)]
    pub dev_show_internal_errors: bool,
}

/// Available top-level commands for the Oneil CLI
#[derive(Subcommand)]
pub enum Commands {
    /// Evaluate an Oneil model
    #[clap(visible_alias = "e")]
    Eval(EvalArgs),

    /// Run tests in an Oneil model
    #[clap(visible_alias = "t")]
    Test(TestArgs),

    /// Print the dependency or reference tree for one or more parameters
    Tree(TreeArgs),

    /// Print the builtins for the Oneil language
    Builtins {
        /// The builtins to print
        #[command(subcommand)]
        command: Option<BuiltinsCommand>,
    },

    /// Print the independent parameters in a model
    Independent(IndependentArgs),

    /// Run the LSP
    Lsp {},

    /// Development tools for debugging and testing Oneil source files
    ///
    /// NOTE: because these commands are not intended for end users, they are hidden
    /// from the help output. However, they can still be used. See `oneil dev --help`
    /// for more information.
    #[clap(hide = true)]
    Dev {
        /// The specific development command to execute
        #[command(subcommand)]
        command: DevCommand,
    },
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "this is a configuration struct for evaluating a model"
)]
#[derive(Args)]
pub struct EvalArgs {
    /// Path to the Oneil model file to evaluate
    #[arg(value_name = "FILE", value_parser = parse_model_path)]
    pub file: ModelPath,

    /// When provided, selects which parameters to print
    ///
    /// The value should be a comma-separated list of parameters. A parameter
    /// may have one or more submodels, separated by a dot. `p.submodel2.submodel1` means the
    /// parameter `p` in `submodel2`, which is in `submodel1`, which
    /// is in the top model.
    ///
    /// When provided, `--print-mode` and `--top-only` are ignored. If both
    /// `--params` and `--exec` are provided, both the parameters and
    /// the expression results are displayed.
    ///
    /// Examples:
    ///
    /// - `--params a` - print the parameter `a` in the top model
    ///
    /// - `--params a,b,c.sub,d` - print the parameters `a`, `b`, and `d` in
    ///   the top model, and the parameter `c` in the submodel `sub`
    ///
    /// - `-p a.submodel2.submodel1` - print the parameter `a` in the submodel `submodel2` in
    ///   the submodel `submodel1` in the top model
    #[arg(long, short = 'p')]
    pub params: Option<VariableList>,

    /// Selects what mode to print the results in
    ///
    /// This can be one of:
    ///
    /// - `trace`: print parameters marked with `*` (trace parameters),
    ///   `**` (debug parameters), or `$` (performance parameters)
    ///
    /// - `perf`: print parameters marked with `$` (performance parameters) only
    ///
    /// - `all`: print all parameter values
    #[arg(long, short = 'P', default_value_t)]
    pub print: PrintMode,

    /// Print debug information
    ///
    /// For parameters marked with `**`, this will print the
    /// values of variables used to evaluate the parameter.
    #[arg(long, short = 'D')]
    pub debug: bool,

    /// Watch files for changes and re-evaluate the model
    #[arg(long)]
    pub watch: bool,

    /// Evaluate expression(s). The expressions are evaluated in the context
    /// of the model being evaluated.
    ///
    /// To convert an expression result to a unit, append `: <unit>` to the expression.
    /// For example: `--expr "distance / time : km/h"`.
    ///
    /// This option can be provided multiple times. Each occurrence accepts
    /// one string.
    ///
    /// If this option is used with `--params`, both the parameters and
    /// the expression results are displayed.
    #[arg(long, short = 'x', value_name = "STRING")]
    pub expr: Vec<String>,

    /// Print info about submodels as well as the top model
    ///
    /// By default, Oneil will only print the results of the top model.
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Display partial results even if there are errors
    ///
    /// If errors occurred during evaluation, errors will be printed,
    /// then the partial results will be printed.
    #[arg(long)]
    pub partial: bool,

    /// Don't print the results header
    #[arg(long)]
    pub no_header: bool,

    /// Don't print the test report
    #[arg(long)]
    pub no_test_report: bool,

    /// Don't print the parameters
    ///
    /// Note that this overrides the `--params` and `--print-mode` options.
    #[arg(long)]
    pub no_parameters: bool,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "this is a configuration struct for running tests in a model"
)]
#[derive(Args)]
pub struct TestArgs {
    /// Path to the Oneil model file to run tests in
    #[arg(value_name = "FILE", value_parser = parse_model_path)]
    pub file: ModelPath,

    /// Print submodel test results recursively
    ///
    /// By default, only the top model test results are printed. When enabled,
    /// submodel test results are also printed.
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Display partial test results even if there are errors
    ///
    /// If errors occurred during evaluation, errors will be printed,
    /// then the partial results will be printed.
    #[arg(long)]
    pub partial: bool,

    /// Don't print the results header
    #[arg(long)]
    pub no_header: bool,

    /// Don't print the test report
    #[arg(long)]
    pub no_test_report: bool,
}

#[derive(Args)]
pub struct TreeArgs {
    /// Path to the Oneil model file to print the tree for
    #[arg(value_name = "FILE", value_parser = parse_model_path)]
    pub file: ModelPath,

    /// The parameter to print the tree for
    #[arg(value_name = "PARAM", required = true)]
    pub params: Vec<ParameterName>,

    /// Print the tree of parameter references
    ///
    /// By default, the tree printed represents the dependencies
    /// of the provided parameters. When enabled, the tree instead
    /// represents parameters where the provided parameters are referenced.
    #[arg(long)]
    pub list_refs: bool,

    /// Print submodel values in the tree
    ///
    /// By default, only the top model values are included in the tree. When enabled,
    /// submodel values are also included in the tree.
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Depth of the tree to print
    ///
    /// By default, the tree is printed to the full depth. When enabled,
    /// the tree is printed to the specified depth.
    #[arg(long)]
    pub depth: Option<usize>,

    /// Display partial trees even if there are errors
    ///
    /// If errors occurred during evaluation, errors will be printed,
    /// then the partial trees will be printed.
    #[arg(long)]
    pub partial: bool,
}

/// Available subcommands for the `Builtins` command
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum BuiltinsCommand {
    /// Print all the builtins
    #[command(name = "all")]
    All,

    /// Print the builtin units or search for a specific unit
    #[command(name = "unit")]
    Units {
        /// The unit to search for
        #[arg(value_name = "UNIT")]
        unit_name: Option<UnitBaseName>,
    },

    /// Print the builtin functions or search for a specific function
    #[command(name = "func")]
    Functions {
        /// The function to search for
        #[arg(value_name = "FUNCTION")]
        function_name: Option<BuiltinFunctionName>,
    },

    /// Print the builtin values or search for a specific value
    #[command(name = "value")]
    Values {
        /// The value to search for
        #[arg(value_name = "VALUE")]
        value_name: Option<BuiltinValueName>,
    },

    /// Print the builtin unit prefixes or search for a specific prefix
    #[command(name = "prefix")]
    Prefixes {
        /// The prefix to search for
        #[arg(value_name = "PREFIX")]
        prefix_name: Option<UnitPrefix>,
    },
}

#[derive(Args)]
pub struct IndependentArgs {
    /// Path to the Oneil model file to print the independent parameters for
    #[arg(value_name = "FILE", value_parser = parse_model_path)]
    pub file: ModelPath,

    /// Print the independent parameters in submodels as well as the top model
    #[arg(long, short = 'r')]
    pub recursive: bool,

    /// Print the parameter values
    #[arg(long)]
    pub values: bool,

    /// Display partial results even if there are errors
    ///
    /// If errors occurred during evaluation, errors will be printed,
    /// then the partial results will be printed.
    #[arg(long)]
    pub partial: bool,
}

/// Development-specific commands for the Oneil CLI
#[expect(
    clippy::enum_variant_names,
    reason = "the names are descriptive and just happen to start with the same word; in the future, other commands may be added that don't start with the same word"
)]
#[derive(Subcommand)]
pub enum DevCommand {
    /// Print the Abstract Syntax Tree (AST) of a Oneil source file
    PrintAst {
        /// Path to the Oneil source file(s) to parse and display
        #[arg(value_name = "FILE", value_parser = parse_model_path)]
        files: Vec<ModelPath>,

        /// Display partial AST even if there are parsing errors
        ///
        /// When enabled, shows the portion of the AST that was successfully
        /// parsed. Useful for debugging incomplete or malformed code.
        #[arg(long)]
        partial: bool,
    },
    /// Print the Intermediate Representation (IR) of a Oneil source file
    PrintIr {
        /// Path to the Oneil source file to process and display
        #[arg(value_name = "FILE", value_parser = parse_model_path)]
        file: ModelPath,

        /// Display partial IR even if there are loading errors
        ///
        /// When enabled, shows the portion of the IR that was successfully generated
        /// before encountering errors. Useful for debugging model loading issues.
        #[arg(long)]
        partial: bool,

        /// Print submodel IR recursively
        ///
        /// By default, only the top model IR is printed. When enabled,
        /// submodel IR is also printed.
        #[arg(long, short = 'r')]
        recursive: bool,

        /// Include only the given parts of the IR (comma-separated)
        ///
        /// Valid values: python, submodels, references, parameters, tests.
        /// If not specified, all parts are shown.
        #[arg(long, value_delimiter(','), value_name = "SECTIONS")]
        include: Option<Vec<IrIncludeSection>>,

        /// Omit parameter values, limits, and test expressions from the output
        #[arg(long)]
        no_values: bool,
    },
    /// Print the results of evaluating an Oneil model
    ///
    /// This prints a debug representation, unlike the `Eval` command,
    /// which is intended to be used by end users.
    PrintModelResult {
        /// Path to the Oneil model file to evaluate
        #[arg(value_name = "FILE", value_parser = parse_model_path)]
        file: ModelPath,

        /// Display partial results even if there are errors
        ///
        /// When enabled, shows the portion of the results that were successfully generated
        /// before encountering errors. Useful for debugging model evaluation issues.
        #[arg(long)]
        partial: bool,

        /// Print submodel and reference results recursively
        ///
        /// By default, only the top model result is printed. When enabled,
        /// nested model results are also printed.
        #[arg(long, short = 'r', default_value_t = false)]
        recursive: bool,

        /// Include only the given parts of the result (comma-separated)
        ///
        /// Valid values: submodels, references, parameters, tests.
        /// If not specified, all parts are shown.
        #[arg(long, value_delimiter(','), value_name = "SECTIONS")]
        include: Option<Vec<ModelResultIncludeSection>>,

        /// Omit parameter values from the output
        #[arg(long)]
        no_values: bool,
    },
    /// Print Python imports from Oneil source file(s)
    #[cfg(feature = "python")]
    PrintPythonImports {
        /// Path(s) to the Oneil source file(s) to inspect
        #[arg(value_name = "FILE", num_args = 1.., value_parser = parse_python_path)]
        files: Vec<PythonPath>,
    },
}

/// Section of the IR that can be selected for `dev print-ir --include`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrIncludeSection {
    PythonImports,
    Submodels,
    References,
    Parameters,
    Tests,
}

impl str::FromStr for IrIncludeSection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "python" => Ok(Self::PythonImports),
            "submodels" => Ok(Self::Submodels),
            "references" => Ok(Self::References),
            "parameters" => Ok(Self::Parameters),
            "tests" => Ok(Self::Tests),
            _ => Err(format!(
                "unknown section \"{s}\"; valid options are: python, submodels, references, parameters, tests"
            )),
        }
    }
}

/// Section of the model result that can be selected for `dev print-model-result --include`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelResultIncludeSection {
    Submodels,
    References,
    Parameters,
    Tests,
}

impl str::FromStr for ModelResultIncludeSection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "submodels" => Ok(Self::Submodels),
            "references" => Ok(Self::References),
            "parameters" => Ok(Self::Parameters),
            "tests" => Ok(Self::Tests),
            _ => Err(format!(
                "unknown section \"{s}\"; valid options are: submodels, references, parameters, tests"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintMode {
    #[default]
    Trace,
    Performance,
    All,
}

impl str::FromStr for PrintMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(Self::All),
            "trace" => Ok(Self::Trace),
            "perf" => Ok(Self::Performance),
            _ => Err("valid options are `all`, `trace`, or `perf`".to_string()),
        }
    }
}

impl fmt::Display for PrintMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Trace => write!(f, "trace"),
            Self::Performance => write!(f, "perf"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableList(Vec<Variable>);

impl VariableList {
    pub fn iter(&self) -> impl Iterator<Item = &Variable> {
        self.0.iter()
    }
}

impl str::FromStr for VariableList {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let params = s
            .split(',')
            .filter_map(|s| (!s.is_empty()).then_some(s.trim().parse::<Variable>()))
            .collect::<Result<_, _>>()?;
        Ok(Self(params))
    }
}

impl fmt::Display for VariableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(Variable::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable(Vec<String>);

impl Variable {
    /// Splits the variable into a vector of strings.
    ///
    /// `param.submodel1.submodel2` becomes `["param", "submodel1", "submodel2"]`.
    pub fn to_vec(&self) -> Vec<String> {
        self.0.clone()
    }
}

impl str::FromStr for Variable {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.split('.').map(str::to_string).collect()))
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("."))
    }
}

/// Parses a CLI argument into a [`ModelPath`].
/// Accepts either a path with `.on` extension or a path with no extension.
fn parse_model_path(s: &str) -> Result<ModelPath, String> {
    let path = Path::new(s);
    match path.extension().and_then(|e| e.to_str()) {
        Some("on") => Ok(ModelPath::from_path_with_ext(path)),
        None => Ok(ModelPath::from_str_no_ext(s)),
        Some(_) => Err(format!(
            "path must have `.on` extension or no extension, got {}",
            path.display()
        )),
    }
}

#[cfg(feature = "python")]
/// Parses a CLI argument into a [`PythonPath`].
/// Accepts either a path with `.py` extension or a path with no extension.
fn parse_python_path(s: &str) -> Result<PythonPath, String> {
    let path = PathBuf::from(s);
    match path.extension().and_then(|e| e.to_str()) {
        Some("py") => Ok(PythonPath::from_path_no_ext(&path.with_extension(""))),
        None => Ok(PythonPath::from_str_no_ext(s)),
        Some(_) => Err(format!(
            "path must have `.py` extension or no extension, got {}",
            path.display()
        )),
    }
}
