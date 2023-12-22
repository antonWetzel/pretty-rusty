use ra_ap_syntax::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::{
    output::{Output, OutputTarget, Priority, Whitespace},
    settings::Settings,
    state::State,
};

pub fn format(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    match node.kind() {
        SyntaxKind::FN => format_function(node, state, settings, output),
        _ => format_default(node, state, settings, output),
    }
}

pub fn format_token(
    token: &SyntaxToken,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    match token.kind() {
        SyntaxKind::WHITESPACE => match token.text().chars().filter(|&c| c == '\n').count() {
            0 => output.set_whitespace(Whitespace::Space, Priority::Low),
            1 => output.set_whitespace(Whitespace::LineBreak, Priority::Low),
            _ => output.set_whitespace(Whitespace::LineBreaks(2), Priority::Low),
        },
        _ => {
            output.raw(token.text(), state, settings);
        }
    }
}

pub fn format_default(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children_with_tokens() {
        match child {
            ra_ap_syntax::NodeOrToken::Node(node) => format(&node, state, settings, output),
            ra_ap_syntax::NodeOrToken::Token(token) => {
                format_token(&token, state, settings, output)
            }
        }
    }
}

pub fn format_function(
    node: &SyntaxNode,
    state: State,
    settings: &Settings,
    output: &mut Output<impl OutputTarget>,
) {
    for child in node.children_with_tokens() {
        match child {
            ra_ap_syntax::NodeOrToken::Node(node) => match node.kind() {
                SyntaxKind::PARAM_LIST => {
                    output.set_whitespace(Whitespace::Space, Priority::Normal);
                    format(&node, state, settings, output);
                    output.set_whitespace(Whitespace::Space, Priority::Normal);
                }
                _ => format(&node, state, settings, output),
            },
            ra_ap_syntax::NodeOrToken::Token(token) => {
                format_token(&token, state, settings, output)
            }
        }
    }
}
