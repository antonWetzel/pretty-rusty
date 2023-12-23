use ra_ap_syntax::SyntaxKind;
use crate::{
	output::{ Output, OutputTarget, Priority, Whitespace },
	settings::{ ListPaddingSettings, Settings },
	state::State,
	Node,
	NodeExtension,
};

pub fn format(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	match node.kind() {
		SyntaxKind::WHITESPACE => format_whitespace(node, state, settings, output),

		// lists
		SyntaxKind::USE_TREE_LIST => format_list(node, state, settings, output, &settings.use_list),
		SyntaxKind::ARG_LIST => format_list(node, state, settings, output, &settings.arguments),
		SyntaxKind::PARAM_LIST => format_list(node, state, settings, output, &settings.parameters),

		// todo: own parameters
		SyntaxKind::TUPLE_FIELD_LIST => format_list(node, state, settings, output, &settings.parameters),
		SyntaxKind::VARIANT_LIST => format_list(node, state, settings, output, &settings.parameters),
		SyntaxKind::RECORD_FIELD_LIST => format_list(node, state, settings, output, &settings.parameters),
		SyntaxKind::TUPLE_EXPR => format_list(node, state, settings, output, &settings.parameters),
		SyntaxKind::TUPLE_PAT => format_list(node, state, settings, output, &settings.parameters),
		SyntaxKind::RECORD_EXPR_FIELD_LIST => format_list(node, state, settings, output, &settings.parameters),

		// top level items
		SyntaxKind::MATCH_ARM_LIST => format_match_arms(node, state, settings, output),
		SyntaxKind::FN => format_top_item(node, state, settings, output),
		SyntaxKind::ENUM => format_top_item(node, state, settings, output),
		SyntaxKind::STRUCT => format_top_item(node, state, settings, output),
		SyntaxKind::TYPE_ALIAS => format_top_item(node, state, settings, output),
		SyntaxKind::IMPL => format_top_item(node, state, settings, output),

		// scopes
		SyntaxKind::STMT_LIST => format_scope(node, state, settings, output),
		SyntaxKind::ITEM_LIST => format_scope(node, state, settings, output),
		SyntaxKind::ASSOC_ITEM_LIST => format_scope(node, state, settings, output),

		// token tree in macros
		SyntaxKind::TOKEN_TREE => skip_formatting(node, state, settings, output),

		// keywords
		SyntaxKind::MATCH_KW => format_keyword(node, state, settings, output),
		SyntaxKind::FN_KW => format_keyword(node, state, settings, output),
		SyntaxKind::PUB_KW => format_keyword(node, state, settings, output),

		// other
		SyntaxKind::COMMA => format_padded_right(node, state, settings, output, Whitespace::Space),
		SyntaxKind::FAT_ARROW => format_padded(node, state, settings, output),
		SyntaxKind::THIN_ARROW => format_padded(node, state, settings, output),
		SyntaxKind::EQ => format_padded(node, state, settings, output),
		SyntaxKind::DOT => format_dot(node, state, settings, output),
		SyntaxKind::SEMICOLON => format_padded_right(node, state, settings, output, Whitespace::LineBreak),

		_ => format_default(node, state, settings, output),
	}
}

pub fn format_default(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	output.raw(node.text(), state, settings);
	for child in node.children() {
		format(child, state, settings, output);
	}
}

fn format_whitespace(
	node: Node,
	_state: State,
	_settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	match node.text().chars().filter(|&c| c == '\n').count() {
		0 => output.set_whitespace(Whitespace::Space, Priority::Low),
		1 => output.set_whitespace(Whitespace::LineBreak, Priority::Normal),
		_ => output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal),
	}
}

fn format_dot(
	node: Node,
	mut state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
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
	padding_settings: &ListPaddingSettings,
) {
	let trailing = node
		.children()
		.fold(false, |trailing, child| match (trailing, child.kind()) {
		(_, SyntaxKind::COMMA) => true,
		(old, SyntaxKind::WHITESPACE) => old,
		(old, kind) if is_close(kind) => old,
		(_, _) => false,
	});
	for child in node.children() {
		match child.kind() {
			kind if is_open(kind) => {
				if padding_settings.before {
					output.set_whitespace(Whitespace::Space, Priority::Normal);
				}
				format(child, state, settings, output);
				if trailing {
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
					state.indent()
				} else if padding_settings.start {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::None, Priority::High);
				}
			}
			kind if is_close(kind) => {
				if trailing {
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
					state.dedent();
				} else if padding_settings.end {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::None, Priority::High);
				}
				format(child, state, settings, output);
				if padding_settings.after {
					output.set_whitespace(Whitespace::Space, Priority::Normal);
				}
			}
			SyntaxKind::COMMA if trailing => {
				format(child, state, settings, output);
				output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
			}
			_ => format(child, state, settings, output),
		}
	}
}

fn format_scope(
	node: Node,
	mut state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	let empty = node.children().all(|node| {
		matches!(node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE)
	});

	for child in node.children() {
		match child.kind() {
			SyntaxKind::L_CURLY => {
				format(child, state, settings, output);
				if empty {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
					state.indent()
				}
			}
			SyntaxKind::R_CURLY => {
				if empty {
					output.set_whitespace(Whitespace::Space, Priority::High);
				} else {
					output.set_whitespace(Whitespace::LineBreak, Priority::High);
					state.dedent();
				}
				format(child, state, settings, output);
			}
			_ => format(child, state, settings, output),
		}
	}
}

fn format_match_arms(
	node: Node,
	mut state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	let trailing = if let Some(last_arm) = node
		.children()
		.filter(|node| node.kind() == SyntaxKind::MATCH_ARM)
		.last()
	{
		last_arm
			.children()
			.fold(false, |trailing, child| match (trailing, child.kind()) {
			(_, SyntaxKind::COMMA) => true,
			(old, SyntaxKind::WHITESPACE) => old,
			(_, _) => false,
		})
	} else {
		false
	};
	for child in node.children() {
		match child.kind() {
			SyntaxKind::L_CURLY if trailing => {
				format(child, state, settings, output);
				output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
				state.indent()
			}
			SyntaxKind::R_CURLY if trailing => {
				output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
				state.dedent();
				format(child, state, settings, output);
			}
			SyntaxKind::MATCH_ARM => {
				format(child, state, settings, output);
				if trailing {
					output.set_whitespace(Whitespace::LineBreak, Priority::Normal);
				} else {
					output.set_whitespace(Whitespace::Space, Priority::Normal);
				}
			}
			_ => format(child, state, settings, output),
		}
	}
}

fn format_top_item(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal);
	format_default(node, state, settings, output);
	output.set_whitespace(Whitespace::LineBreaks(2), Priority::Normal);
}

fn is_open(kind: SyntaxKind) -> bool {
	matches!(
	kind, SyntaxKind::L_PAREN | SyntaxKind::L_BRACK | SyntaxKind::L_CURLY | SyntaxKind::L_ANGLE
	)
}

fn is_close(kind: SyntaxKind) -> bool {
	matches!(
	kind, SyntaxKind::R_PAREN | SyntaxKind::R_BRACK | SyntaxKind::R_CURLY | SyntaxKind::R_ANGLE
	)
}

fn format_padded_right(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
	whitespace: Whitespace,
) {
	output.set_whitespace(Whitespace::None, Priority::Normal);
	format_default(node, state, settings, output);
	output.set_whitespace(whitespace, Priority::Normal);
}

fn format_keyword(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	format_default(node, state, settings, output);
	output.set_whitespace(Whitespace::Space, Priority::High);
}

fn format_padded(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	output.set_whitespace(Whitespace::Space, Priority::Normal);
	format_default(node, state, settings, output);
	output.set_whitespace(Whitespace::Space, Priority::Normal);
}

fn skip_formatting(
	node: Node,
	state: State,
	settings: &Settings,
	output: &mut Output<impl OutputTarget>,
) {
	output.raw(node.text(), state, settings);
	for child in node.children() {
		skip_formatting(child, state, settings, output);
	}
}
