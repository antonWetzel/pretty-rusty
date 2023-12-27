mod logic;
mod output;
pub mod settings;
mod state;


use output::Output;
pub use settings::Settings;
pub use output::Target;
use state::State;

pub use ra_ap_syntax as ast;


pub fn format_node(
	node: ast::SyntaxNode,
	settings: Settings,
	target: &mut impl Target,
) {
	let mut output = Output::new(target);
	let mut state = State::new(settings);
	logic::format_node(&node, ast::SyntaxKind::SOURCE_FILE, &mut state, &mut output);
	output.finish(&state);
}
