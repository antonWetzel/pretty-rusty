use ra_ap_syntax::{SyntaxKind, NodeOrToken, SyntaxNode, SyntaxToken};
use crate::{
	output::{Output, OutputTarget, Whitespace},
	state::State,
};
use SyntaxKind as K;
use Whitespace as W;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
	Default,

	CompactList,
	PaddedList,
	MultilineList,
}


fn format_token(token: &SyntaxToken, state: &mut State, output: &mut Output<impl OutputTarget>) {
	output.text(token.text(), state);
}


fn format_children(node: &SyntaxNode, state: &mut State, scope: Scope, output: &mut Output<impl OutputTarget>) {
	let save = state.save();
	let mut ws = Whitespace::None;
	let mut children = node.children_with_tokens();
	let last = children.find(| node | node.kind() != SyntaxKind::WHITESPACE).unwrap();
	match &last {
		NodeOrToken::Token(token) => format_token(token, state, output),
		NodeOrToken::Node(node) => format_node(node, state, output),
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
				format_node(&child, state, output);
				last = child.kind();
			}
		}
	}

	state.restore(save);
}


fn skip(node: &SyntaxNode, state: &mut State, output: &mut Output<impl OutputTarget>) {
	for child in node.children_with_tokens() {
		match child {
			NodeOrToken::Node(node) => skip(&node, state, output),
			NodeOrToken::Token(token) => output.text(token.text(), state),
		}
	}
}


pub fn format_node(node: &SyntaxNode, state: &mut State, output: &mut Output<impl OutputTarget>) {
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
			| K::RECORD_EXPR_FIELD_LIST
			| K::ARRAY_EXPR
			| K::GENERIC_ARG_LIST
			=> list(node, false),

		K::ATTR | K::PATH_SEGMENT | K::PAREN_EXPR | K::PAREN_PAT => Scope::CompactList,

		K::MATCH_ARM_LIST => Scope::MultilineList,

		K::STMT_LIST | K::ITEM_LIST | K::ASSOC_ITEM_LIST => code(node),

		_ => Scope::Default,
	};

	format_children(node, state, scope, output);
}


fn whitespace(
	left: SyntaxKind,
	middle: Whitespace,
	right: SyntaxKind,
	scope: Scope,
	parent: SyntaxKind,
	state: &mut State,
) -> Whitespace {
	match (left, middle, right) {
		// comments
		(K::COMMENT, ws, _) => ws,

		// list open
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE, _, _) if scope == Scope::MultilineList => {
			state.indent();
			W::LineBreak
		}
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE, _, _) if scope == Scope::PaddedList => {
			state.enter_scope();
			W::Space
		}
		(K::L_PAREN | K::L_BRACK | K::L_CURLY | K::L_ANGLE, _, _) if scope == Scope::CompactList => {
			state.enter_scope();
			W::None
		}

		// list close
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) if scope == Scope::MultilineList => {
			state.dedent();
			W::LineBreak
		}
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) if scope == Scope::PaddedList => W::Space,
		(_, _, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) if scope == Scope::CompactList => W::None,

		// list seperator
		(_, _, K::COMMA) => W::None,
		(K::COMMA, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::COMMA, _, _) if scope == Scope::MultilineList => W::LineBreak,

		//top items
		(K::USE, _, K::USE) => W::LineBreak,
		(K::MODULE, _, K::MODULE) => W::LineBreak,
		(
			_,
			_,
			K::USE | K::MODULE | K::FN | K::STRUCT | K::IMPL | K::ENUM | K::UNION | K::MACRO_RULES | K::MACRO_CALL,
		) => W::LineBreaks(3),
		(
			K::USE | K::MODULE | K::FN | K::STRUCT | K::IMPL | K::ENUM | K::UNION | K::MACRO_RULES | K::MACRO_CALL,
			_,
			_,
		) => W::LineBreaks(3),
		(K::ATTR, _, _) => W::LineBreak,

		// statements
		(K::EXPR_STMT | K::LET_STMT, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::EXPR_STMT | K::LET_STMT, _, _) if scope == Scope::MultilineList => W::LineBreak,

		// chains
		(_, W::LineBreak | W::LineBreaks(_), K::DOT | K::PIPE | K::FAT_ARROW) => {
			state.start_chain();
			W::LineBreak
		}

		// tokens
		(_, _, K::COLON2) => W::None,
		(K::COLON2, _, _) => W::None,

		(_, _, K::COLON) => W::None,
		(K::COLON, _, _) => W::Space,

		(_, _, K::SEMICOLON) => W::None,
		(_, _, K::QUESTION) => W::None,

		(K::POUND, _, _) => W::None,
		(K::BANG, _, _) if matches!(parent, SyntaxKind::ATTR | SyntaxKind::MACRO_CALL) => W::None,
		(_, _, K::BANG) => W::None,
		(K::AMP, _, _) => W::None,

		(_, _, K::DOT) => W::None,
		(K::DOT, _, _) => W::None,

		(_, _, K::EQ) => W::Space,
		(K::EQ, _, _) => W::Space,

		// other
		(_, _, K::META) => W::None,
		(K::META, _, _) => W::None,
		(K::PATH, _, _) if matches!(parent, K::META | K::TUPLE_STRUCT_PAT) => W::None,
		(K::NAME | K::NAME_REF, _, K::PARAM_LIST | K::GENERIC_ARG_LIST) => W::None,

		(K::MATCH_ARM, W::LineBreaks(_), _) if scope == Scope::MultilineList => W::LineBreaks(2),
		(K::MATCH_ARM, _, _) if scope == Scope::MultilineList => W::LineBreak,

		(_, _, K::ARG_LIST) => W::None,

		(_, _, _) => W::Space,
	}
}


fn list(
	node: &SyntaxNode,
	pad: bool,
) -> Scope {
	let trailing = node
		.children_with_tokens()
		.fold(false, | trailing, child | match (trailing, child.kind()) {
			(_, K::COMMA | K::SEMICOLON) => true,
			(old, K::WHITESPACE) => old,
			(old, K::R_PAREN | K::R_BRACK | K::R_CURLY | K::R_ANGLE) => old,
			(_, _) => false,
		});

	match (trailing, pad) {
		(true, _) => Scope::MultilineList,
		(_, true) => Scope::PaddedList,
		(_, false) => Scope::CompactList,
	}
}


fn code(
	node: &SyntaxNode,
) -> Scope {
	let empty = node.children_with_tokens().all(| node | {
		matches!(node.kind(), SyntaxKind::L_CURLY | SyntaxKind::R_CURLY | SyntaxKind::WHITESPACE)
	});
	match empty {
		true => Scope::PaddedList,
		false => Scope::MultilineList,
	}
}
