use std::ops::Not;

use ra_ap_syntax::SyntaxKind;
use crate::{
	output::{ Output, OutputTarget, Priority, Whitespace },
	settings::Settings,
	state::State,
	Node,
	NodeExtension,
};

use Whitespace as W;
use Priority as P;
use SyntaxKind as K;

pub fn format(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	let (before, after, priority) = match node.kind() {
		K::WHITESPACE => return format_whitespace(node, state, settings, output),
		K::ERROR => return skip_formatting(node, state, settings, output),

		// lists
		K::USE_TREE_LIST => return format_list(node, state, settings, output, settings.pad_curly_braces),
		K::ARG_LIST => return format_list(node, state, settings, output, settings.pad_parenthesis),
		K::PARAM_LIST => return format_list(node, state, settings, output, settings.pad_parenthesis),
		K::GENERIC_PARAM_LIST => return format_list(node, state, settings, output, settings.pad_angled_brackets),
		K::TUPLE_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_parenthesis),
		K::VARIANT_LIST => return format_list(node, state, settings, output, settings.pad_curly_braces),
		K::RECORD_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_curly_braces),
		K::TUPLE_EXPR => return format_list(node, state, settings, output, settings.pad_parenthesis),
		K::TUPLE_PAT => return format_list(node, state, settings, output, settings.pad_parenthesis),
		K::RECORD_EXPR_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_curly_braces),
		K::ARRAY_EXPR => return format_list(node, state, settings, output, settings.pad_square_brackets),

		// top level items
		K::MATCH_ARM_LIST => return format_match_arms(node, state, settings, output),
		K::FN => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::Normal),
		K::ENUM => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::Normal),
		K::STRUCT => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::Normal),
		K::TYPE_ALIAS => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::Normal),
		K::IMPL => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::Normal),

		// scopes
		K::STMT_LIST => return format_code_scope(node, state, settings, output),
		K::ITEM_LIST => return format_code_scope(node, state, settings, output),
		K::ASSOC_ITEM_LIST => return format_code_scope(node, state, settings, output),

		// token tree in macros
		K::TOKEN_TREE => return skip_formatting(node, state, settings, output),

		// keywords
		K::MATCH_KW => (None, Some(W::Space), P::High),
		K::FN_KW => (None, Some(W::Space), P::High),
		K::PUB_KW => (None, Some(W::Space), P::High),
		K::RETURN_KW => (None, Some(W::Space), P::High),
		K::IMPL_KW => (None, Some(W::Space), P::High),
		K::WHERE_KW => (None, Some(W::Space), P::High),
		K::FOR_KW => (None, Some(W::Space), P::High),
		K::LOOP_KW => (None, Some(W::Space), P::High),

		// single tokens
		K::FAT_ARROW => (Some(W::Space), Some(W::Space), P::High),
		K::THIN_ARROW => (Some(W::Space), Some(W::Space), P::High),
		K::EQ => (Some(W::Space), Some(W::Space), P::High),
		K::COLON => (Some(W::None), Some(W::Space), P::Normal),
		K::AMP => (None, Some(W::None), P::Normal),
		K::COLON2 => (Some(W::None), Some(W::None), P::Normal),

		// chain tokens
		K::DOT => return format_dot(node, state, settings, output),

		// end of file
		K::EOF if settings.final_newline => (None, Some(W::LineBreak), P::Guaranteed),
		K::EOF => (None, Some(W::None), P::Guaranteed),

		// nothing special for rest
		_ => return format_default(node, state, settings, output),
	};
	format_padded(node, state, settings, output, before, after, priority);
}

pub fn format_default(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	output.raw(node.text(), state, settings);
	for child in node.children() {
		format(child, state, settings, output);
	}
}

fn format_whitespace(node: Node, _state: &mut State, _settings: &Settings, output: &mut Output<impl OutputTarget>) {
	match node.text().chars().filter(|&c| c == '\n').count() {
		0 => output.set_whitespace(Whitespace::Space, Priority::Low),
		1 => output.set_whitespace(Whitespace::LineBreak, Priority::Low),
		_ => output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal),
	}
}

fn format_list(
	node: Node,
	state: &mut State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
	pad: bool,
) {
	let trailing = node
	.children()
	.fold(false, |trailing, child| match (trailing, child.kind()) {
		(_, K::COMMA | K::SEMICOLON) => true,
		(old, K::WHITESPACE) => old,
		(old, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) => old,
		(_, _) => false,
	});

	format_items(node, state, settings, output, trailing, pad);
}

fn format_dot(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	match output.get_whitespace().0 {
		Whitespace::LineBreak | Whitespace::LineBreaks(_) => {
			state.indent();
		}
		_ => output.set_whitespace(Whitespace::None, Priority::High),
	}
	format_default(node, state, settings, output);
	output.set_whitespace(Whitespace::None, Priority::High);
}

fn format_items(
	node: Node,
	state: &mut State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,

	mutlti_line: bool,
	pad: bool,
) {
	let indentation = state.indentation;
	for child in node.children() {
		match child.kind() {
			K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE => {
				format(child, state, settings, output);
				if mutlti_line {
					state.indent();
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
				} else if pad {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::None, Priority::High);
				}
			}
			K::COMMA | K::SEMICOLON => {
				format(child, state, settings, output);
				if mutlti_line {
					output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
				} else {
					output.set_whitespace(Whitespace::Space, Priority::High);
				}
			}
			K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE => {
				if mutlti_line {
					state.dedent();
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
				} else if pad {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::None, Priority::High);
				}
				format(child, state, settings, output);
			},
			_ => format(child, state, settings, output),
		}
	}
	state.indentation = indentation;
}

fn format_code_scope(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	let empty = node.children().all(|node| {
		matches!(node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE)
	});
	format_items(node, state, settings, output, empty.not(), true);
}

fn format_match_arms(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	let trailing = if let Some(last_arm) = node
	.children()
	.filter(|node| node.kind() == SyntaxKind::MATCH_ARM)
	.last() {
		last_arm
		.children()
		.fold(false, |trailing, child| match (
			trailing,
			child.kind(),
		) {
			(_, SyntaxKind::COMMA) => true,
			(old, SyntaxKind::WHITESPACE) => old,
			(_, _) => false,
		})
	} else {
		false
	};
	format_items(node, state, settings, output, trailing, true);
}

fn format_padded(
	node: Node,
	state: &mut State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,

	before: Option<Whitespace>,
	after: Option<Whitespace>,
	priority: Priority,
) {
	if let Some(before) = before {
		output.set_whitespace(before, priority);
	}
	format_default(node, state, settings, output);
	if let Some(after) = after {
		output.set_whitespace(after, priority);
	}
}

fn skip_formatting(node: Node, state: &mut State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	output.raw(node.text(), state, settings);
	for child in node.children() {
		skip_formatting(child, state, settings, output);
	}
}
