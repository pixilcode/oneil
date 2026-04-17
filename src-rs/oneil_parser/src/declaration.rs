//! Parser for declarations in an Oneil program.

use nom::{
    Parser as _,
    branch::alt,
    combinator::{all_consuming, opt},
    multi::many0,
};

use oneil_ast::{
    Decl, DeclNode, DesignParameter, DesignTarget, Directory, DirectoryNode, IdentifierNode,
    Import, ModelInfo, ModelInfoNode, ModelKind, Node, SubmodelList, SubmodelListNode, UseDesign,
    UseModel,
};
use oneil_shared::span::Span;

use crate::{
    error::{ParserError, parser_trait::ErrorHandlingParser},
    note::parse as parse_note,
    parameter::{parse as parse_parameter, parse_parameter_value},
    test::parse as parse_test,
    token::{
        keyword::{as_, design, for_, import, ref_, use_, with},
        naming::identifier,
        structure::end_of_line,
        symbol::{bracket_left, bracket_right, comma, dot, dot_dot, equals, slash},
    },
    util::{InputSpan, Result},
};

/// Parses a complete `design <model>` declaration (for single-decl entry points).
pub fn parse_design_target_complete(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    all_consuming(parse_design_target_line).parse(input)
}

/// Parses a declaration at the top level of a model or inside a `section`.
///
/// When `allow_design_shorthand` is `true` (after a successful `design <model>` line in a `.one`
/// bundle), design-body shorthand lines are parsed before ordinary parameters.
pub fn parse(
    input: InputSpan<'_>,
    allow_design_shorthand: bool,
) -> Result<'_, DeclNode, ParserError> {
    decl_inner(input, allow_design_shorthand)
}

/// Parses a declaration (full input).
pub fn parse_complete(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    all_consuming(|i| parse(i, false)).parse(input)
}

/// Parses any type of declaration by trying each declaration parser in sequence.
///
/// This function attempts to parse the input as each type of declaration:
/// 1. Import declaration (`import path`)
/// 2. Use design declaration (`use design file [for alias]`)
/// 3. Design target probe (returns error if `design` appears in wrong context)
/// 4. Use declaration (`use path [as alias] [with submodels]`)
/// 5. Test declaration (`test: condition`)
/// 6. Design parameter shorthand (`id = value`, only when `allow_design_shorthand` is true)
/// 7. Parameter declaration (parameter definitions)
///
/// The first parser that succeeds determines the declaration type. If a keyword
/// matches but the rest of the declaration is malformed, an error is returned
/// rather than trying subsequent parsers.
fn decl_inner(
    input: InputSpan<'_>,
    allow_design_shorthand: bool,
) -> Result<'_, DeclNode, ParserError> {
    let import_start = input.location_offset();
    match import_decl.parse(input) {
        Ok(v) => return Ok(v),
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        Err(nom::Err::Error(e)) => {
            if e.error_offset > import_start {
                return Err(nom::Err::Error(e));
            }
        }
        Err(nom::Err::Failure(e)) => {
            if e.error_offset > import_start {
                return Err(nom::Err::Failure(e));
            }
        }
    }

    match use_design_decl.parse(input) {
        Ok(v) => return Ok(v),
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        Err(nom::Err::Error(_) | nom::Err::Failure(_)) => {}
    }

    match parse_design_target_line.parse(input) {
        Ok((_, node)) => {
            let err = if input.extra.allow_design_header {
                ParserError::design_header_duplicate(node.span())
            } else {
                ParserError::design_header_wrong_file(node.span())
            };
            return Err(nom::Err::Error(err));
        }
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        Err(nom::Err::Error(_) | nom::Err::Failure(_)) => {}
    }

    let use_decl_start = input.location_offset();
    match use_decl.parse(input) {
        Ok(v) => return Ok(v),
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        Err(nom::Err::Error(e)) => {
            // If `use`/`ref` matched but the declaration is invalid, propagate the
            // error — do not fall through to `test`/`parameter`.
            if e.error_offset > use_decl_start {
                return Err(nom::Err::Error(e));
            }
        }
        Err(nom::Err::Failure(e)) => {
            if e.error_offset > use_decl_start {
                return Err(nom::Err::Failure(e));
            }
        }
    }
    match test_decl.parse(input) {
        Ok(v) => return Ok(v),
        Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
        Err(nom::Err::Error(_) | nom::Err::Failure(_)) => {}
    }

    if allow_design_shorthand {
        // Then try design parameter (`id = value` or `id.instance = value`)
        match design_parameter_decl.parse(input) {
            Ok(v) => return Ok(v),
            Err(nom::Err::Incomplete(n)) => return Err(nom::Err::Incomplete(n)),
            Err(nom::Err::Error(_) | nom::Err::Failure(_)) => {}
        }
    }

    parameter_decl
        .convert_error_to(ParserError::expect_decl)
        .parse(input)
}

/// Parses an import declaration
fn import_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, import_token) = import.convert_errors().parse(input)?;

    // TODO: allow a path here (ex. `import foo.bar`)
    let (rest, import_path_token) = identifier
        .or_fail_with(ParserError::import_missing_path(import_token.lexeme_span))
        .parse(rest)?;

    let (rest, end_of_line_token) = end_of_line
        .or_fail_with(ParserError::import_missing_end_of_line(
            import_path_token.lexeme_span,
        ))
        .parse(rest)?;

    let node_span =
        Span::from_start_and_end(&import_token.lexeme_span, &end_of_line_token.lexeme_span);
    let whitespace_span = end_of_line_token.whitespace_span;

    let import_path_str = Node::<String>::from(import_path_token);

    let import_node = Node::new(Import::new(import_path_str), node_span, whitespace_span);

    let decl_node = Node::new(Decl::Import(import_node), node_span, whitespace_span);

    Ok((rest, decl_node))
}

/// Parses a top-level `design [path/to/]<model>` line (`.one` design bundles and wrong-file probe on `.on`).
pub fn parse_design_target_line(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, design_token) = design.convert_errors().parse(input)?;

    // Parse optional directory path (e.g., `../models/`)
    let (rest, directory_path) = opt_directory_path.parse(rest)?;

    let (rest, target_token) = identifier
        .or_fail_with(ParserError::design_missing_target(design_token.lexeme_span))
        .parse(rest)?;

    let (rest, end_of_line_token) = end_of_line
        .or_fail_with(ParserError::import_missing_end_of_line(
            target_token.lexeme_span,
        ))
        .parse(rest)?;

    let target_node = IdentifierNode::from(target_token);
    let node_span =
        Span::from_start_and_end(&design_token.lexeme_span, &end_of_line_token.lexeme_span);
    let whitespace_span = end_of_line_token.whitespace_span;
    let inner = if directory_path.is_empty() {
        Node::new(DesignTarget::new(target_node), node_span, whitespace_span)
    } else {
        Node::new(
            DesignTarget::with_path(directory_path, target_node),
            node_span,
            whitespace_span,
        )
    };
    let decl_node = Node::new(Decl::DesignTarget(inner), node_span, whitespace_span);

    Ok((rest, decl_node))
}

/// Parses `use design [path/to/]<file> [for <alias>]`.
fn use_design_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, use_token) = use_.convert_errors().parse(input)?;
    let (rest, design_token) = design.convert_errors().parse(rest)?;

    // Parse optional directory path (e.g., `../designs/`)
    let (rest, directory_path) = opt_directory_path.parse(rest)?;

    let (rest, file_token) = identifier
        .or_fail_with(ParserError::use_design_missing_file(
            design_token.lexeme_span,
        ))
        .parse(rest)?;

    let (rest, instance) = opt(|input| {
        let (rest, _for_token) = for_.convert_errors().parse(input)?;
        let (rest, alias_token) = identifier.convert_errors().parse(rest)?;
        Ok((rest, IdentifierNode::from(alias_token)))
    })
    .parse(rest)?;

    let file_node = IdentifierNode::from(file_token);
    let final_span = instance
        .as_ref()
        .map_or_else(|| file_node.span(), Node::span);

    let (rest, end_of_line_token) = end_of_line
        .or_fail_with(ParserError::use_missing_end_of_line(final_span))
        .parse(rest)?;

    let node_span =
        Span::from_start_and_end(&use_token.lexeme_span, &end_of_line_token.lexeme_span);
    let whitespace_span = end_of_line_token.whitespace_span;
    let inner = if directory_path.is_empty() {
        Node::new(
            UseDesign::new(file_node, instance),
            node_span,
            whitespace_span,
        )
    } else {
        Node::new(
            UseDesign::with_path(directory_path, file_node, instance),
            node_span,
            whitespace_span,
        )
    };
    let decl_node = Node::new(Decl::UseDesign(inner), node_span, whitespace_span);

    Ok((rest, decl_node))
}

/// Parses `id = value` or `id.instance = value` in a design file (after `design`).
fn design_parameter_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, ident_token) = identifier.convert_errors().parse(input)?;
    let ident_node = IdentifierNode::from(ident_token);

    // Optionally parse `.instance` for scoped parameter overrides
    let (rest, instance_node) = opt(|input| {
        let (rest, _dot_token) = dot.convert_errors().parse(input)?;
        let (rest, instance_token) = identifier.convert_errors().parse(rest)?;
        Ok((rest, IdentifierNode::from(instance_token)))
    })
    .parse(rest)?;

    let (rest, equals_token) = equals.convert_errors().parse(rest)?;

    let (rest, value_node) = parse_parameter_value
        .or_fail_with(ParserError::parameter_missing_value(
            equals_token.lexeme_span,
        ))
        .parse(rest)?;

    let (rest, linebreak_token) = end_of_line
        .or_fail_with(ParserError::parameter_missing_end_of_line(
            value_node.span(),
        ))
        .parse(rest)?;

    let (rest, note_node) = opt(parse_note).parse(rest)?;

    let param_start_span = ident_node.span();
    let (param_end_span, param_whitespace_span) = note_node.as_ref().map_or(
        (linebreak_token.lexeme_span, linebreak_token.whitespace_span),
        |note_node| (note_node.span(), note_node.whitespace_span()),
    );

    let param_span = Span::from_start_and_end(&param_start_span, &param_end_span);
    let inner = if let Some(instance) = instance_node {
        DesignParameter::with_instance(ident_node, instance, value_node, note_node)
    } else {
        DesignParameter::new(ident_node, value_node, note_node)
    };
    let inner_node = Node::new(inner, param_span, param_whitespace_span);
    let decl_node = Node::new(
        Decl::DesignParameter(inner_node),
        param_span,
        param_whitespace_span,
    );

    Ok((rest, decl_node))
}

/// Parses a use declaration
fn use_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let ref_keyword = |input| {
        let (rest, ref_token) = ref_.convert_errors().parse(input)?;
        Ok((rest, (ModelKind::Reference, ref_token)))
    };

    let use_keyword = |input| {
        let (rest, use_token) = use_.convert_errors().parse(input)?;
        Ok((rest, (ModelKind::Submodel, use_token)))
    };

    // either parse the ref keyword or the use keyword
    let (rest, (is_ref_only, keyword_token)) = alt((ref_keyword, use_keyword)).parse(input)?;

    let (rest, directory_path) = opt_directory_path.parse(rest)?;

    let (rest, model_info) = model_info_simple
        .or_fail_with(ParserError::use_missing_model_info(
            keyword_token.lexeme_span,
        ))
        .parse(rest)?;

    let (rest, submodel_list) = opt(|input| {
        let (rest, _with_token) = with.convert_errors().parse(input)?;
        submodel_list(rest)
    })
    .parse(rest)?;

    let final_span = submodel_list
        .as_ref()
        .map_or_else(|| model_info.span(), Node::span);

    let (rest, end_of_line_token) = end_of_line
        .or_fail_with(ParserError::use_missing_end_of_line(final_span))
        .parse(rest)?;

    let use_model_node = Node::new(
        UseModel::new(directory_path, model_info, submodel_list, is_ref_only),
        final_span,
        end_of_line_token.whitespace_span,
    );

    let decl_node = Node::new(
        Decl::UseModel(use_model_node),
        final_span,
        end_of_line_token.whitespace_span,
    );

    Ok((rest, decl_node))
}

/// Parses a directory path in a model path
fn opt_directory_path(input: InputSpan<'_>) -> Result<'_, Vec<DirectoryNode>, ParserError> {
    many0(|input| {
        let (rest, directory_name) = directory_name.parse(input)?;
        let (rest, _slash_token) = slash.convert_errors().parse(rest)?;
        Ok((rest, directory_name))
    })
    .parse(input)
}

/// Parses a directory name in a model path
fn directory_name(input: InputSpan<'_>) -> Result<'_, DirectoryNode, ParserError> {
    let directory_name = |input| {
        let (rest, directory_name_token) = identifier.convert_errors().parse(input)?;
        let directory_name = DirectoryNode::from(directory_name_token);
        Ok((rest, directory_name))
    };

    let current_directory = |input| {
        let (rest, dot_token) = dot.convert_errors().parse(input)?;
        let current_directory = dot_token.into_node_with_value(Directory::current());
        Ok((rest, current_directory))
    };

    let parent_directory = |input| {
        let (rest, dot_dot_token) = dot_dot.convert_errors().parse(input)?;
        let parent_directory = dot_dot_token.into_node_with_value(Directory::parent());
        Ok((rest, parent_directory))
    };

    alt((directory_name, current_directory, parent_directory)).parse(input)
}

/// Parses a model info without subcomponents (for the main model in `use`/`ref`).
fn model_info_simple(input: InputSpan<'_>) -> Result<'_, ModelInfoNode, ParserError> {
    let (rest, top_component_token) = identifier.convert_errors().parse(input)?;
    let top_component_node = IdentifierNode::from(top_component_token);

    let (rest, alias) = opt(as_alias).parse(rest)?;

    let (final_span, whitespace_span) = alias.as_ref().map_or_else(
        || {
            (
                top_component_node.span(),
                top_component_node.whitespace_span(),
            )
        },
        |a| (a.span(), a.whitespace_span()),
    );

    let model_info_span = Span::from_start_and_end(&top_component_node.span(), &final_span);
    let model_info = ModelInfo::new(top_component_node, vec![], alias);

    Ok((
        rest,
        Node::new(model_info, model_info_span, whitespace_span),
    ))
}

/// Parses a model info with optional subcomponents (for submodels in `with` clause).
pub fn model_info(input: InputSpan<'_>) -> Result<'_, ModelInfoNode, ParserError> {
    let (rest, top_component_token) = identifier.convert_errors().parse(input)?;
    let top_component_node = IdentifierNode::from(top_component_token);

    let (rest, subcomponents) = opt_subcomponents.parse(rest)?;
    let (rest, alias) = opt(as_alias).parse(rest)?;

    let (final_span, whitespace_span) = match (subcomponents.last(), &alias) {
        (_, Some(alias)) => (alias.span(), alias.whitespace_span()),
        (Some(subcomponent), None) => (subcomponent.span(), subcomponent.whitespace_span()),
        (None, None) => (
            top_component_node.span(),
            top_component_node.whitespace_span(),
        ),
    };

    let model_info_span = Span::from_start_and_end(&top_component_node.span(), &final_span);
    let model_info = ModelInfo::new(top_component_node, subcomponents, alias);

    Ok((
        rest,
        Node::new(model_info, model_info_span, whitespace_span),
    ))
}

fn opt_subcomponents(input: InputSpan<'_>) -> Result<'_, Vec<IdentifierNode>, ParserError> {
    many0(|input| {
        let (rest, dot_token) = dot.convert_errors().parse(input)?;

        let (rest, subcomponent_token) = identifier
            .or_fail_with(ParserError::model_path_missing_subcomponent(
                dot_token.lexeme_span,
            ))
            .parse(rest)?;

        let subcomponent_node = IdentifierNode::from(subcomponent_token);

        Ok((rest, subcomponent_node))
    })
    .parse(input)
}

/// Parses an alias identifier after an `as` keyword.
fn as_alias(input: InputSpan<'_>) -> Result<'_, IdentifierNode, ParserError> {
    let (rest, as_token) = as_.convert_errors().parse(input)?;

    let (rest, alias_token) = identifier
        .or_fail_with(ParserError::as_missing_alias(as_token.lexeme_span))
        .parse(rest)?;

    let alias_node = IdentifierNode::from(alias_token);

    Ok((rest, alias_node))
}

/// Parses a list of submodels in a use declaration
fn submodel_list(input: InputSpan<'_>) -> Result<'_, SubmodelListNode, ParserError> {
    let single_submodel = |input| {
        let (rest, submodel) = model_info.parse(input)?;
        let submodel_span = submodel.span();
        let submodel_whitespace_span = submodel.whitespace_span();

        let submodel_list = SubmodelList::new(vec![submodel]);
        let submodel_list_node = Node::new(submodel_list, submodel_span, submodel_whitespace_span);

        Ok((rest, submodel_list_node))
    };

    let multiple_submodels = |input| {
        let (rest, bracket_left_token) = bracket_left.convert_errors().parse(input)?;

        let (rest, _optional_end_of_line_token) = opt(end_of_line).convert_errors().parse(rest)?;

        let (rest, submodel_list) = opt(|input| {
            let (rest, first_submodel) = model_info.parse(input)?;

            let (rest, rest_submodels) = many0(|input| {
                let (rest, _comma_token) = comma.convert_errors().parse(input)?;
                let (rest, _optional_end_of_line_token) =
                    opt(end_of_line).convert_errors().parse(rest)?;
                // Normally, this `submodel` parsing would have `or_fail_with`
                // since we have found a comma token. However, the comma may be
                // the optional trailing comma, so we don't fail here.
                let (rest, submodel) = model_info.parse(rest)?;
                Ok((rest, submodel))
            })
            .parse(rest)?;

            let (rest, _optional_trailing_comma_token) = opt(comma).convert_errors().parse(rest)?;
            let (rest, _optional_end_of_line_token) =
                opt(end_of_line).convert_errors().parse(rest)?;

            let mut submodels = rest_submodels;
            submodels.insert(0, first_submodel);
            Ok((rest, submodels))
        })
        .parse(rest)?;

        let (rest, bracket_right_token) = bracket_right
            .or_fail_with(ParserError::unclosed_bracket(
                bracket_left_token.lexeme_span,
            ))
            .parse(rest)?;

        let submodel_list = SubmodelList::new(submodel_list.unwrap_or_default());
        let submodel_list_span = Span::from_start_and_end(
            &bracket_left_token.lexeme_span,
            &bracket_right_token.lexeme_span,
        );
        let submodel_list_whitespace_span = bracket_right_token.whitespace_span;

        let submodel_list_node = Node::new(
            submodel_list,
            submodel_list_span,
            submodel_list_whitespace_span,
        );

        Ok((rest, submodel_list_node))
    };

    alt((single_submodel, multiple_submodels)).parse(input)
}

/// Parses a parameter declaration by delegating to the parameter parser.
fn parameter_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, parameter) = parse_parameter.parse(input)?;

    let parameter_span = parameter.span();
    let parameter_whitespace_span = parameter.whitespace_span();
    let decl_node = Node::new(
        Decl::Parameter(parameter),
        parameter_span,
        parameter_whitespace_span,
    );

    Ok((rest, decl_node))
}

/// Parses a test declaration by delegating to the test parser.
fn test_decl(input: InputSpan<'_>) -> Result<'_, DeclNode, ParserError> {
    let (rest, test) = parse_test.parse(input)?;

    let span = test.span();
    let whitespace_span = test.whitespace_span();
    let decl_node = Node::new(Decl::Test(test), span, whitespace_span);

    Ok((rest, decl_node))
}

#[cfg(test)]
#[expect(
    clippy::similar_names,
    reason = "tests should make it clear what variable is being tested"
)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::util::test::assert_node_contains;

    mod success {
        use super::*;

        #[test]
        fn import_decl() {
            let input = InputSpan::new_extra("import foo\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::Import(ref import_node) = *decl else {
                panic!("Expected import declaration");
            };

            let import_path = import_node.path();
            assert_node_contains!(import_path, "foo".to_string(), start_offset: 7, end_offset: 10);

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn ref_decl() {
            let input = InputSpan::new_extra("ref foo\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");
            assert_eq!(use_model_node.model_kind(), ModelKind::Reference);

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl() {
            let input = InputSpan::new_extra("use foo with bar as baz\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");
            assert_eq!(use_model_node.model_kind(), ModelKind::Submodel);

            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].top_component().as_str(), "bar");
            assert_eq!(submodels[0].get_alias().as_str(), "baz");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_without_alias() {
            let input = InputSpan::new_extra("use foo with bar\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].top_component().as_str(), "bar");
            assert_eq!(submodels[0].get_alias().as_str(), "bar");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_simple_without_alias() {
            let input = InputSpan::new_extra("use foo\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn parse_complete_import_success() {
            let input = InputSpan::new_extra("import foo\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::Import(ref import_node) = *decl else {
                panic!("Expected import declaration");
            };

            let import_path_node = import_node.path();
            assert_node_contains!(import_path_node, "foo", start_offset: 7, end_offset: 10);

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn parse_complete_use_success() {
            let input = InputSpan::new_extra("use foo with bar as baz\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].get_alias().as_str(), "baz");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_single_directory() {
            let input = InputSpan::new_extra("use utils/math as calculator\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "math");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "calculator");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "utils");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_single_directory_without_alias() {
            let input = InputSpan::new_extra("use utils/math\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "math");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "math");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "utils");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_multiple_directories() {
            let input = InputSpan::new_extra(
                "use models/physics/mechanics as dynamics\n",
                Config::default(),
            );
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "mechanics");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "dynamics");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 2);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "models");
            assert_eq!(use_model_node.directory_path()[1].as_str(), "physics");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_directory_and_subcomponents() {
            let input = InputSpan::new_extra(
                "use utils/math with trigonometry as trig\n",
                Config::default(),
            );
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "math");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "math");

            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].top_component().as_str(), "trigonometry");
            assert_eq!(submodels[0].get_alias().as_str(), "trig");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "utils");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_current_directory() {
            let input = InputSpan::new_extra("use ./local_model as local\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "local_model");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "local");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), ".");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_parent_directory() {
            let input = InputSpan::new_extra("use ../parent_model as parent\n", Config::default());
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "parent_model");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "parent");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "..");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_mixed_directory_types() {
            let input = InputSpan::new_extra(
                "use ../shared/./utils/math as shared_math\n",
                Config::default(),
            );
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "math");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "shared_math");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 4);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "..");
            assert_eq!(use_model_node.directory_path()[1].as_str(), "shared");
            assert_eq!(use_model_node.directory_path()[2].as_str(), ".");
            assert_eq!(use_model_node.directory_path()[3].as_str(), "utils");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_with_complex_path_and_subcomponents() {
            let input = InputSpan::new_extra(
                "use models/physics/mechanics with rotational.dynamics as rotation\n",
                Config::default(),
            );
            let (rest, decl) = parse_complete(input).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "mechanics");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "mechanics");

            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].top_component().as_str(), "rotational");
            assert_eq!(submodels[0].subcomponents().len(), 1);
            assert_eq!(submodels[0].subcomponents()[0].as_str(), "dynamics");
            assert_eq!(submodels[0].get_alias().as_str(), "rotation");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 2);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "models");
            assert_eq!(use_model_node.directory_path()[1].as_str(), "physics");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn directory_name_parsing() {
            // Test parent directory
            let input = InputSpan::new_extra("..", Config::default());
            let (rest, dir) = directory_name(input).expect("should parse parent directory");
            assert_eq!(dir.as_str(), "..");
            assert_eq!(rest.fragment(), &"");

            // Test current directory
            let input = InputSpan::new_extra(".", Config::default());
            let (rest, dir) = directory_name(input).expect("should parse current directory");
            assert_eq!(dir.as_str(), ".");
            assert_eq!(rest.fragment(), &"");

            // Test regular directory name
            let input = InputSpan::new_extra("foo", Config::default());
            let (rest, dir) = directory_name(input).expect("should parse regular directory name");
            assert_eq!(dir.as_str(), "foo");
            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn mixed_directory_path_parsing() {
            let input = InputSpan::new_extra("../shared/./utils/", Config::default());
            let (_rest, directory_path) =
                opt_directory_path(input).expect("should parse mixed directory path");

            assert_eq!(directory_path.len(), 4);
            assert_eq!(*directory_path[0], Directory::Parent);
            assert_eq!(*directory_path[1], Directory::Name("shared".to_string()));
            assert_eq!(*directory_path[2], Directory::Current);
            assert_eq!(*directory_path[3], Directory::Name("utils".to_string()));
        }

        #[test]
        fn use_decl_with_submodel_with_subcomponents() {
            let input = InputSpan::new_extra("use foo with bar.qux\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 1);

            let submodel = &submodels[0];
            assert_eq!(submodel.top_component().as_str(), "bar");
            assert_eq!(submodel.subcomponents().len(), 1);
            assert_eq!(submodel.subcomponents()[0].as_str(), "qux");
            assert_eq!(submodel.get_alias().as_str(), "qux");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_multiple_submodels() {
            let input = InputSpan::new_extra("use foo with [bar, qux]\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "bar");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "bar");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "qux");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "qux");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_multiple_submodels_with_aliases() {
            let input = InputSpan::new_extra(
                "use foo with [bar as baz, qux as quux]\n",
                Config::default(),
            );
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "bar");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "baz");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "qux");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "quux");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_multiple_submodels_with_subcomponents() {
            let input =
                InputSpan::new_extra("use foo with [bar.qux, baz.quux.quuz]\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "bar");
            assert_eq!(submodel1.subcomponents().len(), 1);
            assert_eq!(submodel1.subcomponents()[0].as_str(), "qux");
            assert_eq!(submodel1.get_alias().as_str(), "qux");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "baz");
            assert_eq!(submodel2.subcomponents().len(), 2);
            assert_eq!(submodel2.subcomponents()[0].as_str(), "quux");
            assert_eq!(submodel2.subcomponents()[1].as_str(), "quuz");
            assert_eq!(submodel2.get_alias().as_str(), "quuz");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_multiple_submodels_with_trailing_comma() {
            let input = InputSpan::new_extra("use foo with [bar, qux,]\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "bar");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "bar");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "qux");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "qux");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_empty_submodel_list() {
            let input = InputSpan::new_extra("use foo with []\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels - should be empty
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 0);

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_model_alias_and_submodels() {
            let input = InputSpan::new_extra("use foo as bar with [qux, baz]\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "bar");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "qux");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "qux");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "baz");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "baz");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_complex_path_and_submodels() {
            let input = InputSpan::new_extra(
                "use utils/math with [trigonometry as trig, sin, cos as cosine]\n",
                Config::default(),
            );
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "math");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "math");

            // Check directory path
            assert_eq!(use_model_node.directory_path().len(), 1);
            assert_eq!(use_model_node.directory_path()[0].as_str(), "utils");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 3);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "trigonometry");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "trig");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "sin");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "sin");

            let submodel3 = &submodels[2];
            assert_eq!(submodel3.top_component().as_str(), "cos");
            assert_eq!(submodel3.subcomponents().len(), 0);
            assert_eq!(submodel3.get_alias().as_str(), "cosine");

            assert_eq!(rest.fragment(), &"");
        }

        #[test]
        fn use_decl_with_submodels_and_newlines() {
            let input = InputSpan::new_extra("use foo with [\nbar,\nqux\n]\n", Config::default());
            let (rest, decl) = parse(input, false).expect("parsing should succeed");

            let Decl::UseModel(ref use_model_node) = *decl else {
                panic!("Expected use declaration");
            };

            let use_model_info = use_model_node.model_info();
            assert_eq!(use_model_info.top_component().as_str(), "foo");
            assert_eq!(use_model_info.subcomponents().len(), 0);
            assert_eq!(use_model_info.get_alias().as_str(), "foo");

            // Check submodels
            let submodels = use_model_node
                .imported_submodels()
                .expect("should have submodels");
            assert_eq!(submodels.len(), 2);

            let submodel1 = &submodels[0];
            assert_eq!(submodel1.top_component().as_str(), "bar");
            assert_eq!(submodel1.subcomponents().len(), 0);
            assert_eq!(submodel1.get_alias().as_str(), "bar");

            let submodel2 = &submodels[1];
            assert_eq!(submodel2.top_component().as_str(), "qux");
            assert_eq!(submodel2.subcomponents().len(), 0);
            assert_eq!(submodel2.get_alias().as_str(), "qux");

            assert_eq!(rest.fragment(), &"");
        }
    }

    mod error {
        use std::ops::Deref;

        use crate::error::reason::{
            DeclKind, ExpectKind, ImportKind, IncompleteKind, ParserErrorReason, UseKind,
        };
        use crate::token::error::{ExpectKind as TokenExpectKind, TokenErrorKind};

        use super::*;

        /// Asserts that `parse(input_str)` returns `Err(Failure(...))` with the
        /// given `IncompleteKind` and cause span.
        fn assert_failure(
            input_str: &str,
            error_offset: usize,
            expected_kind: IncompleteKind,
            cause_start: usize,
            cause_end: usize,
        ) {
            let input = InputSpan::new_extra(input_str, Config::default());
            let error = match parse(input, false) {
                Err(nom::Err::Failure(e)) | Err(nom::Err::Error(e)) => e,
                Ok(_) => panic!("Expected error for {input_str:?}"),
                Err(e) => panic!("Unexpected nom result for {input_str:?}: {e:?}"),
            };
            assert_eq!(error.error_offset, error_offset, "offset for {input_str:?}");
            let ParserErrorReason::Incomplete { kind, cause } = error.reason else {
                panic!(
                    "Expected Incomplete for {input_str:?}, got {:?}",
                    error.reason
                );
            };
            assert_eq!(kind, expected_kind, "kind for {input_str:?}");
            assert_eq!(
                cause.start().offset,
                cause_start,
                "cause_start for {input_str:?}"
            );
            assert_eq!(cause.end().offset, cause_end, "cause_end for {input_str:?}");
        }

        #[test]
        fn expect_decl_errors() {
            let cases: &[&str] = &[
                "",
                "foo\n",
                "   \n",
                "# comment\n",
                "invalid syntax\n",
                "impor\n",
                "export foo\n",
                "Import foo\n",
                "+++---\n",
                "123 456\n",
            ];
            for input_str in cases {
                let input = InputSpan::new_extra(input_str, Config::default());
                let error = match parse(input, false) {
                    Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => e,
                    Ok(_) => panic!("Expected error for {input_str:?}"),
                    Err(e) => panic!("Unexpected nom result for {input_str:?}: {e:?}"),
                };
                assert_eq!(
                    error.reason,
                    ParserErrorReason::Expect(ExpectKind::Decl),
                    "reason for {input_str:?}"
                );
            }
        }

        #[test]
        fn import_incomplete_errors() {
            use ImportKind::*;
            let cases: &[(&str, usize, ImportKind, usize, usize)] = &[
                ("import\n", 6, MissingPath, 0, 6),
                ("import 123\n", 7, MissingPath, 0, 6),
                ("import foo@bar\n", 10, MissingEndOfLine, 7, 10),
            ];
            for &(input_str, offset, ref import_kind, cs, ce) in cases {
                assert_failure(
                    input_str,
                    offset,
                    IncompleteKind::Decl(DeclKind::Import(*import_kind)),
                    cs,
                    ce,
                );
            }
        }

        #[test]
        fn use_incomplete_errors() {
            let cases: &[(&str, usize, IncompleteKind, usize, usize)] = &[
                (
                    "use foo with bar as\n",
                    19,
                    IncompleteKind::Decl(DeclKind::AsMissingAlias),
                    17,
                    19,
                ),
                (
                    "use 123 with bar as baz\n",
                    4,
                    IncompleteKind::Decl(DeclKind::Use(UseKind::MissingModelInfo)),
                    0,
                    3,
                ),
                (
                    "use foo with bar as 123\n",
                    20,
                    IncompleteKind::Decl(DeclKind::AsMissingAlias),
                    17,
                    19,
                ),
            ];
            for &(input_str, offset, ref expected_kind, cs, ce) in cases {
                assert_failure(input_str, offset, *expected_kind, cs, ce);
            }
        }

        #[test]
        fn model_info_expect_identifier_errors() {
            let cases: &[&str] = &["", "123.bar"];
            for input_str in cases {
                let input = InputSpan::new_extra(input_str, Config::default());
                let Err(nom::Err::Error(error)) = model_info(input) else {
                    panic!("Expected Error for {input_str:?}");
                };
                assert_eq!(error.error_offset, 0, "offset for {input_str:?}");
                assert_eq!(
                    error.reason,
                    ParserErrorReason::TokenError(TokenErrorKind::Expect(
                        TokenExpectKind::Identifier
                    )),
                    "reason for {input_str:?}"
                );
            }
        }

        #[test]
        fn model_info_missing_subcomponent() {
            let cases: &[(&str, usize, usize, usize)] = &[
                ("foo.", 4, 3, 4),
                ("foo.123", 4, 3, 4),
                ("foo.bar.", 8, 7, 8),
            ];
            for &(input_str, offset, cs, ce) in cases {
                let input = InputSpan::new_extra(input_str, Config::default());
                let Err(nom::Err::Failure(error)) = model_info(input) else {
                    panic!("Expected Failure for {input_str:?}");
                };
                assert_eq!(error.error_offset, offset, "offset for {input_str:?}");
                let ParserErrorReason::Incomplete { kind, cause } = error.reason else {
                    panic!(
                        "Expected Incomplete for {input_str:?}, got {:?}",
                        error.reason
                    );
                };
                assert_eq!(
                    kind,
                    IncompleteKind::Decl(DeclKind::ModelMissingSubcomponent),
                    "kind for {input_str:?}"
                );
                assert_eq!(cause.start().offset, cs, "cause_start for {input_str:?}");
                assert_eq!(cause.end().offset, ce, "cause_end for {input_str:?}");
            }
        }

        #[test]
        fn parse_complete_with_remaining_input() {
            let input = InputSpan::new_extra("import foo\nrest", Config::default());
            let result = parse_complete(input);

            let Err(nom::Err::Error(error)) = result else {
                panic!("Unexpected result {result:?}");
            };

            assert_eq!(error.error_offset, 11);
            assert_eq!(error.reason, ParserErrorReason::UnexpectedToken);
        }

        #[test]
        fn unclosed_bracket_errors() {
            // (input, error_offset, cause_start, cause_end)
            let cases: &[(&str, usize, usize, usize)] = &[
                ("use foo with [\n", 15, 13, 14),
                ("use foo with [bar\n", 18, 13, 14),
                ("use foo with [bar, baz\n", 23, 13, 14),
                ("use foo with [bar, baz,\n", 24, 13, 14),
                ("use foo with [bar.qux, baz.quux\n", 32, 13, 14),
                ("use foo with [bar as baz, qux as quux\n", 38, 13, 14),
                ("use foo as bar with [qux, baz\n", 30, 20, 21),
                ("use foo with [\nbar,\nbaz\n", 24, 13, 14),
                (
                    "use utils/math with [trigonometry as trig, sin, cos as cosine\n",
                    62,
                    20,
                    21,
                ),
            ];
            for &(input_str, offset, cs, ce) in cases {
                assert_failure(input_str, offset, IncompleteKind::UnclosedBracket, cs, ce);
            }
        }

        #[test]
        fn use_model_parses_in_design_context() {
            // In design context (allow_design_shorthand=true), `use model as alias`
            // parses as UseModel (resolver determines it's a replacement)
            let input = InputSpan::new_extra("use foo as bar\n", Config::default());
            let result = decl_inner(input, true);
            let Ok((rest, decl)) = result else {
                panic!("Expected Ok, got {result:?}");
            };
            assert!(
                rest.fragment().is_empty() || rest.fragment().chars().all(|c| c.is_whitespace())
            );
            let decl_inner = decl.deref();
            assert!(
                matches!(decl_inner, Decl::UseModel(_)),
                "Expected UseModel, got {:?}",
                decl_inner
            );
        }

        #[test]
        fn use_model_parses_in_regular_context() {
            // In regular context (allow_design_shorthand=false), `use model as alias`
            // parses as UseModel (model import)
            let input = InputSpan::new_extra("use foo as bar\n", Config::default());
            let result = decl_inner(input, false);
            let Ok((rest, decl)) = result else {
                panic!("Expected Ok, got {result:?}");
            };
            assert!(
                rest.fragment().is_empty() || rest.fragment().chars().all(|c| c.is_whitespace())
            );
            let decl_inner = decl.deref();
            assert!(
                matches!(decl_inner, Decl::UseModel(_)),
                "Expected UseModel, got {:?}",
                decl_inner
            );
        }

        #[test]
        fn use_model_with_submodels_single() {
            // Test `use model as alias with submodel` syntax
            let input = InputSpan::new_extra("use foo as bar with baz\n", Config::default());
            let result = decl_inner(input, true);
            let Ok((rest, decl)) = result else {
                panic!("Expected Ok, got {result:?}");
            };
            assert!(
                rest.fragment().is_empty() || rest.fragment().chars().all(|c| c.is_whitespace())
            );
            let Decl::UseModel(um) = decl.deref() else {
                panic!("Expected UseModel, got {:?}", decl.deref());
            };
            assert_eq!(um.model_info().top_component().as_str(), "foo");
            assert_eq!(um.model_info().get_alias().as_str(), "bar");
            let submodels = um.imported_submodels().expect("Expected submodel list");
            assert_eq!(submodels.len(), 1);
            assert_eq!(submodels[0].get_model_name().as_str(), "baz");
        }

        #[test]
        fn use_model_with_submodels_list() {
            // Test `use model as alias with [submodel1, submodel2]` syntax
            let input = InputSpan::new_extra("use foo as bar with [baz, qux]\n", Config::default());
            let result = decl_inner(input, true);
            let Ok((rest, decl)) = result else {
                panic!("Expected Ok, got {result:?}");
            };
            assert!(
                rest.fragment().is_empty() || rest.fragment().chars().all(|c| c.is_whitespace())
            );
            let Decl::UseModel(um) = decl.deref() else {
                panic!("Expected UseModel, got {:?}", decl.deref());
            };
            assert_eq!(um.model_info().top_component().as_str(), "foo");
            assert_eq!(um.model_info().get_alias().as_str(), "bar");
            let submodels = um.imported_submodels().expect("Expected submodel list");
            assert_eq!(submodels.len(), 2);
            assert_eq!(submodels[0].get_model_name().as_str(), "baz");
            assert_eq!(submodels[1].get_model_name().as_str(), "qux");
        }

        #[test]
        fn use_model_without_submodels() {
            // Test that use model without `with` has no submodel list
            let input = InputSpan::new_extra("use foo as bar\n", Config::default());
            let result = decl_inner(input, true);
            let Ok((_, decl)) = result else {
                panic!("Expected Ok, got {result:?}");
            };
            let Decl::UseModel(um) = decl.deref() else {
                panic!("Expected UseModel, got {:?}", decl.deref());
            };
            assert!(um.imported_submodels().is_none());
        }
    }
}
