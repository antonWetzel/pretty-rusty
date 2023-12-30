use crate::{
	ast::{ SyntaxKind, NodeOrToken, SyntaxNode, SyntaxToken },
	output::{ Output, Target, Whitespace },
	state::State,
};

use SyntaxKind as K;
use Whitespace as W;
use ra_ap_syntax::TextSize;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
	Default,

	CompactList,
	PaddedList,
	MultilineList,
}


fn format_token(token: &SyntaxToken, state: &mut State, output: &mut Output<impl Target>) {
	output.text(token.text(), state);
}


fn format_children(node: &SyntaxNode, state: &mut State, parent: SyntaxKind, scope: Scope, output: &mut Output<impl Target>) {
	let save = state.save();
	let mut ws = Whitespace::None;
	let mut children = node.children_with_tokens();
	let last = children.find(|node| node.kind() != SyntaxKind::WHITESPACE).unwrap();
	match &last {
		NodeOrToken::Token(token) => format_token(token, state, output),
		NodeOrToken::Node(node) => format_node(node, node.kind(), state, output),
	}
	let mut last = last.kind();
	for child in children {
		match child {
			NodeOrToken::Token(token) => {
				if let SyntaxKind::WHITESPACE = token.kind() {
					ws = Whitespace::from_text(token.text());
					continue;
				}
				let ws = whitespace(
					last,
					std::mem::take(&mut ws),
					token.kind(),
					scope,
					node.kind(),
					state,
				);
				output.whitespace(ws, state);
				format_token(&token, state, output);
				last = token.kind();
			}
			NodeOrToken::Node(child) => {
				let ws = whitespace(
					last,
					std::mem::take(&mut ws),
					child.kind(),
					scope,
					node.kind(),
					state,
				);
				output.whitespace(ws, state);
				format_node(&child, node.kind(), state, output);
				last = child.kind();
			}
		}
	}
	match parent {
		K::LET_STMT | K::IF_EXPR => { },
		_ => state.restore(save),
	}
}


fn skip(node: &SyntaxNode, state: &mut State, output: &mut Output<impl Target>) {
	for child in node.children_with_tokens() {
		match child {
			NodeOrToken::Node(node) => skip(&node, state, output),
			NodeOrToken::Token(token) => output.text(token.text(), state),
		}
	}
}


pub fn format_node(node: &SyntaxNode, parent: SyntaxKind, state: &mut State, output: &mut Output<impl Target>) {
	let scope = match node.kind() {
		K::ERROR | K::TOKEN_TREE => return skip(node, state, output),

		K::USE_TREE_LIST
			| K::ARG_LIST
			| K::PARAM_LIST
			| K::GENERIC_PARAM_LIST
			| K::TUPLE_FIELD_LIST
			| K::VARIANT_LIST
			| K::RECORD_FIELD_LIST
			| K::TUPLE_EXPR
			| K::TUPLE_TYPE
			| K::TUPLE_STRUCT_PAT
			| K::TUPLE_PAT
			| K::INDEX_EXPR
			| K::RECORD_EXPR_FIELD_LIST
			| K::SLICE_PAT
			| K::SLICE_TYPE
			| K::ARRAY_TYPE
			| K::ARRAY_EXPR
			| K::GENERIC_ARG_LIST
			| K::PAREN_TYPE
			| K::VISIBILITY => {
			let pad = node.children_with_tokens().find_map(|node| match node.kind() {
				SyntaxKind::L_PAREN => Some(state.settings().pad_parenthesis),
				SyntaxKind::L_BRACK => Some(state.settings().pad_square_brackets),
				SyntaxKind::L_CURLY => Some(state.settings().pad_curly_braces),
				SyntaxKind::L_ANGLE => Some(state.settings().pad_angled_brackets),
				_ => None,
			}).unwrap_or_default();
			list(node, pad)
		}

		K::ATTR | K::PATH_SEGMENT | K::PAREN_EXPR | K::PAREN_PAT => Scope::CompactList,

		K::MATCH_ARM_LIST | K::WHERE_CLAUSE => Scope::MultilineList,

		K::ITEM_LIST | K::ASSOC_ITEM_LIST => code(node, 0),
		K::STMT_LIST => code(node, 5),

		_ => Scope::Default,
	};

	format_children(node, state, parent, scope, output);
}


fn top_level(kind: SyntaxKind) -> bool {
	matches!(kind, K::ASSOC_ITEM_LIST | K::ITEM_LIST |  K::SOURCE_FILE)
}


fn whitespace(
	left: SyntaxKind,
	middle: Whitespace,
	right: SyntaxKind,
	scope: Scope,
	parent: SyntaxKind,
	state: &mut State,
) -> Whitespace {
	let ws = match (left, middle, right) {
		// list open
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE | K::PIPE, _, _) if scope == Scope::MultilineList => {
			state.indent();
			W::LineBreak
		}
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE | K::PIPE, _, _) if scope == Scope::PaddedList => {
			state.enter_scope();
			W::Space
		}
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE | K::PIPE, _, _) if scope == Scope::CompactList => {
			state.enter_scope();
			W::None
		}

		// list close
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE | K::PIPE) if scope == Scope::MultilineList => {
			state.dedent();
			W::LineBreak
		}
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE | K::PIPE) if scope == Scope::PaddedList => W::Space,
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE | K::PIPE) if scope == Scope::CompactList => W::None,

		// list seperator
		(_, _, K::COMMA) => W::None,
		(K::COMMA, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::COMMA, _, _) if scope == Scope::MultilineList => W::LineBreak,
		(K::COMMA, _, _) => W::Space,

		//top items
		(K::USE | K::CONST | K::TYPE_ALIAS, W::LineBreaks(_), K::USE | K::CONST | K::TYPE_ALIAS) if left == right => W::LineBreaks(2),
		(K::USE | K::CONST | K::TYPE_ALIAS, _, K::USE | K::CONST | K::TYPE_ALIAS) if left == right => W::LineBreak,

		(K::MODULE, _, K::MODULE) => W::LineBreak,
		(_, _, K::USE | K::MODULE | K::FN | K::STRUCT | K::IMPL | K::ENUM | K::UNION | K::MACRO_RULES | K::MACRO_CALL | K::TYPE_ALIAS | K::TRAIT)
			=> W::LineBreaks(3),
		(K::USE | K::MODULE | K::FN | K::STRUCT | K::IMPL | K::ENUM | K::UNION | K::MACRO_RULES | K::MACRO_CALL | K::TYPE_ALIAS | K::TRAIT, _, _)
			=> W::LineBreaks(3),

		(_, _, K::CONST) if top_level(parent) => W::LineBreaks(3),
		(K::CONST, _, _) if top_level(parent) => W::LineBreaks(3),

		// statements
		(K::EXPR_STMT | K::LET_STMT, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::EXPR_STMT | K::LET_STMT, _, _) if scope == Scope::MultilineList => W::LineBreak,

		// chains
		(_, W::LineBreak | W::LineBreaks(_), K::EQ | K::DOT | K::PIPE | K::PIPE2 | K::AMP2 | K::FAT_ARROW | K::PLUS | K::MINUS | K::STAR | K::SLASH | K::PERCENT | K::AMP | K::CARET | K::SHL | K::SHR | K::FOR_KW) => {
			state.start_chain();
			W::LineBreak
		}

		(_, _, K::ASSOC_ITEM_LIST) if state.in_chain() => {
			state.exit_chain();
			W::LineBreak
		}

		// tokens
		(_, _, K::COLON2 | K::DOT2 | K::DOT2EQ) => W::None,
		(K::COLON2 | K::DOT2 | K::DOT2EQ, _, _) => W::None,

		(_, _, K::COLON) => W::None,
		(K::COLON, _, _) => W::Space,

		(_, _, K::SEMICOLON | K::QUESTION) => W::None,

		(K::POUND, _, _) => W::None,
		(K::BANG, _, _) if matches!(parent, K::ATTR | K::MACRO_CALL) => W::None,
		(_, _, K::BANG) => W::None,
		(K::AMP, _, _) if matches!(parent, K::SELF_PARAM | K::REF_TYPE | K::REF_EXPR | K::REF_PAT) => W::None,

		(_, _, K::DOT) => W::None,
		(K::DOT, _, _) => W::None,

		(_, _, K::EQ) => W::Space,
		(K::EQ, _, _) => W::Space,

		(_, _, K::L_PAREN) => W::None,

		// other
		(K::ATTR, _, _) if matches!(parent, K::FN | K::STRUCT | K::ENUM | K::UNION | K::VARIANT | K::RECORD_FIELD | K::MACRO_RULES)
			=> W::LineBreak,
		(_, _, K::META) => W::None,
		(K::META, _, _) => W::None,
		(K::PATH, _, _) if matches!(parent, K::META | K::TUPLE_STRUCT_PAT) => W::None,

		(K::MATCH_ARM, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::MATCH_ARM, _, _) if scope == Scope::MultilineList => W::LineBreak,

		(_, _, K::PARAM_LIST | K::TUPLE_FIELD_LIST | K::GENERIC_PARAM_LIST | K::GENERIC_ARG_LIST) if parent != K::CLOSURE_EXPR => W::None,
		(K::STAR, _, K::CONST_KW) => W::None,
		(K::STAR | K::PLUS | K::MINUS | K::BANG, _, _) if matches!(parent, K::PREFIX_EXPR | K::PTR_TYPE) => W::None,
		(_, _, K::L_BRACK) if parent == K::INDEX_EXPR => W::None,

		(_, _, K::ARG_LIST) => W::None,

		(_, _, K::WHERE_CLAUSE) => W::LineBreak,
		(K::WHERE_CLAUSE, _, _) => W::LineBreak,
		(_, _, K::WHERE_PRED) => {
			state.start_chain();
			W::LineBreak
		}

		(_, _, K::LET_ELSE | K::BLOCK_EXPR) if state.in_chain() => {
			state.exit_chain();
			W::LineBreak
		}

		(_, W::LineBreaks(_), _) if parent == K::STMT_LIST => W::LineBreaks(2),
		(_, _, _) if parent == K::STMT_LIST => W::LineBreak,
		(_, _, _) => W::Space,
	};

	match (left, middle, right) {
		(_, W::LineBreaks(_), K::COMMENT) => W::LineBreaks(2),
		(_, _, K::COMMENT) => middle,
		(K::COMMENT, W::LineBreaks(_), _) => W::LineBreaks(2),
		(K::COMMENT, W::LineBreak, _) => W::LineBreak,
		(_, _, _) => ws
	}
}


fn list(node: &SyntaxNode, pad: bool) -> Scope {
	if node.kind() == SyntaxKind::RECORD_EXPR_FIELD_LIST {
		if node.children_with_tokens().any(|node| node.kind() == SyntaxKind::DOT2) {
			return Scope::MultilineList;
		}
	}
	let trailing = node
		.children_with_tokens()
		.fold(false, |trailing, child| match (trailing, child.kind()) {
			(_, K::COMMA | K::SEMICOLON) => true,
			(old, K::WHITESPACE | K::COMMENT) => old,
			(old, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) => old,
			(_, _) => false,
		});

	match (trailing, pad) {
		(true, _) => Scope::MultilineList,
		(_, true) => Scope::PaddedList,
		(_, false) => Scope::CompactList,
	}
}


fn code(node: &SyntaxNode, mut max_length: usize) -> Scope {
	let empty = node.children_with_tokens().all(|node| {
		matches!(node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE) || node.text_range().len() < TextSize::try_from(std::mem::take(&mut max_length)).unwrap()
	});

	match empty {
		true => Scope::PaddedList,
		false => Scope::MultilineList,
	}
}
