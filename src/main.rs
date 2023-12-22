mod logic;
mod output;
mod settings;
mod state;

use std::io::BufWriter;

use output::{Output, OutputTarget};
use ra_ap_syntax::{
    ast::{self},
    AstNode, SyntaxNode,
};
use settings::Settings;
use state::State;

fn main() {
    let data = include_str!("../test/test.rs");

    let ast = ast::SourceFile::parse(data).tree();

    let settings = Settings { indentation: 0 };
    let mut target = BufWriter::new(std::io::stdout());
    format_node(ast.syntax(), &settings, &mut target).unwrap();
    drop(target);
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("Failed to read configuration file")]
    FailedToReadConfigurationFile(std::io::Error),
    #[error("malformed configuration file: {0}")]
    MalformatedConfigurationFile(#[from] toml::de::Error),
}

pub fn format_node(
    node: &SyntaxNode,
    settings: &Settings,
    target: &mut impl OutputTarget,
) -> Result<(), FormatError> {
    let mut output = Output::new(target);
    let state = State::new();
    logic::format(node, state, settings, &mut output);

    output.finish(state, settings);
    Ok(())
}
