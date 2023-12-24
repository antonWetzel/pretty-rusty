use std::ops::Not;

use ra_ap_syntax::SyntaxKind;
use crate::{
	output::{ Output, OutputTarget, Priority, Whitespace },
	settings::Settings,
	state::{ State, Scope },
	Node,
	NodeExtension,
};

pub fn format(node: Node, mut state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	use Whitespace as W;
	use Priority as P;
	use SyntaxKind as K;

	let (before, after, priority) = match node.kind() {
		K::WHITESPACE => return format_whitespace(node, state, settings, output),
		K::ERROR => return skip_formatting(node, state, settings, output),

		// lists
		K::USE_TREE_LIST => return format_list(node, state, settings, output, settings.pad_use_list),
		K::ARG_LIST => return format_list(node, state, settings, output, settings.pad_arguments),
		K::PARAM_LIST => return format_list(node, state, settings, output, settings.pad_parameters),
		K::GENERIC_PARAM_LIST => return format_list(node, state, settings, output, settings.pad_parameters),

		// todo: own setting
		K::TUPLE_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_parameters),
		K::VARIANT_LIST => return format_list(node, state, settings, output, settings.pad_parameters),
		K::RECORD_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_parameters),
		K::TUPLE_EXPR => return format_list(node, state, settings, output, settings.pad_parameters),
		K::TUPLE_PAT => return format_list(node, state, settings, output, settings.pad_parameters),
		K::RECORD_EXPR_FIELD_LIST => return format_list(node, state, settings, output, settings.pad_parameters),
		K::ARRAY_EXPR => return format_list(node, state, settings, output, settings.pad_parameters),

		// top level items
		K::MATCH_ARM_LIST => return format_match_arms(node, state, settings, output),
		K::FN => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::High),
		K::ENUM => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::High),
		K::STRUCT => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::High),
		K::TYPE_ALIAS => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::High),
		K::IMPL => (Some(W::LineBreaks(2)), Some(W::LineBreaks(2)), Priority::High),

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
		K::DOT => return format_dot(node, state, settings, output),

		// open list
		K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE => match state.scope {
			Scope::CompactList => (None, Some(W::None), P::High),
			Scope::PaddedList => (None, Some(W::Space), P::High),
			Scope::MultilinList => (None, Some(W::LineBreak), P::High),
			_ => return format_default(node, state, settings, output),
		},

		// seperators
		K::COMMA | K::SEMICOLON => match state.scope {
			Scope::MultilinList => (Some(W::None), Some(W::LineBreak), P::Normal),
			Scope::CompactList => (Some(W::None), Some(W::Space), P::High),
			Scope::PaddedList => (Some(W::None), Some(W::Space), P::High),
			_ => (Some(W::None), None, P::High),
		},

		// close list
		K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE => match state.scope {
			Scope::CompactList => (Some(W::None), None, P::High),
			Scope::PaddedList => (Some(W::Space), None, P::High),
			Scope::MultilinList => {
				state.dedent();
				(Some(W::LineBreak), None, Priority::High)
			}
			_ => return format_default(node, state, settings, output),
		},

		// end of file
		K::EOF if settings.final_newline => (None, Some(W::LineBreak), P::Guaranteed),
		K::EOF => (None, Some(W::None), P::Guaranteed),

		// other
		K::ARRAY_TYPE => return format_scoped(node, state, settings, output, Scope::CompactList),
		K::MACRO_ARM if state.scope == Scope::MultilinList => (None, Some(W::LineBreak), P::High),
		K::MACRO_ARM => (None, Some(W::Space), P::High),

		// nothing special for rest
		_ => return format_default(node, state, settings, output),
	};
	format_padded(node, state, settings, output, before, after, priority);
}

pub fn format_default(node: Node, mut state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	output.raw(node.text(), state, settings);
	state.scope = Scope::Default;
	for child in node.children() {
		format(child, state, settings, output);
	}
}

fn format_scoped(
	node: Node,
	mut state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,

	scope: Scope,
) {
	state.scope = scope;
	output.raw(node.text(), state, settings);
	for child in node.children() {
		format(child, state, settings, output);
	}
}

fn format_whitespace(node: Node, _state: State, _settings: &Settings, output: &mut Output<impl OutputTarget>) {
	match node.text().chars().filter(|&c| c == '\n').count() {
		0 => output.set_whitespace(Whitespace::Space, Priority::Low),
		1 => output.set_whitespace(Whitespace::LineBreak, Priority::Low),
		_ => output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal),
	}
}

fn format_dot(node: Node, mut state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	match output.get_whitespace().0 {
		Whitespace::LineBreak | Whitespace::LineBreaks(_) => state.indent(),
		_ => output.set_whitespace(Whitespace::None, Priority::High),
	}
	format_default(node, state, settings, output);
	output.set_whitespace(Whitespace::None, Priority::High);
}

fn format_list(
	node: Node,
	mut state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
	pad: bool,
) {
	let trailing = node
		.children()
		.fold(false, |trailing, child| match (trailing, child.kind()) {
		(_, SyntaxKind::COMMA) => true,
		(old, SyntaxKind::WHITESPACE) => old,
		(old, SyntaxKind::R_PAREN | SyntaxKind::R_BRACK | SyntaxKind::R_CURLY | SyntaxKind::R_ANGLE) => old,
		(_, _) => false,
	});
	if trailing {
		state.indent()
	}
	format_scoped(node, state, settings, output, match (trailing, pad) {
		(true, _) => Scope::MultilinList,
		(_, true) => Scope::PaddedList,
		(_, false) => Scope::CompactList,
	});
}

fn format_code_scope(node: Node, mut state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	let empty = node.children().all(|node| {
		matches!(node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE)
	});
	if empty.not() {
		state.indent();
	}
	format_scoped(node, state, settings, output, match empty {
		true => Scope::PaddedList,
		false => Scope::MultilinList,
	});
}

fn format_match_arms(node: Node, mut state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
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
	if trailing {
		state.indent();
	}
	format_scoped(node, state, settings, output, match trailing {
		true => Scope::MultilinList,
		false => Scope::PaddedList,
	});
}

fn format_padded(
	node: Node,
	state: State,
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

fn skip_formatting(node: Node, state: State, settings: &Settings, output: &mut Output<impl OutputTarget>) {
	output.raw(node.text(), state, settings);
	for child in node.children() {
		skip_formatting(child, state, settings, output);
	}
}
