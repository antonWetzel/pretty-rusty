mod logic;
mod output;
mod settings;
mod state;
use clap::Parser;
use std::{
	fs::File,
	io::{ BufWriter, Read },
	path::PathBuf,
	ops::Not,
};
use output::{ Output, OutputTarget };
use ra_ap_syntax::{
	ast::{ self },
	NodeOrToken,
	SyntaxNode,
	SyntaxToken,
};
use settings::Settings;
use state::State;

const CONFIG_NAME: &str = "pretty-rusty.toml";

#[derive(Debug, Clone, Parser)]
pub struct Command {
	/// Input path for source file, used as output path if nothing else is specified
	#[arg(default_value = None)]
	pub path: Option<PathBuf>,

	/// Output path
	#[arg(short, long, default_value = None)]
	pub output: Option<PathBuf>,

	/// Search for 'pretty-rusty.toml' for additional formatting settings
	#[arg(long, default_value_t = false)]
	pub use_configuration: bool,

	/// Generate file with formatting settings based on the style
	#[arg(long, default_value_t = false)]
	pub save_configuration: bool,

	/// Use standard input as source
	#[arg(long, default_value_t = false)]
	pub use_std_in: bool,

	/// Use standard output as target
	#[arg(long, default_value_t = false)]
	pub use_std_out: bool,

	/// File location to search for configuration, defaults to input path if available
	#[arg(long, default_value = None)]
	pub file_location: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum FormatError {
	#[error("Failed to get project folder")]
	FailedToGetProjectFolder,

	#[error("Failed to get working directory")]
	FailedToGetWorkingDirectory(std::io::Error),

	#[error("No configuration file")]
	NoConfigurationFile,

	#[error("Failed to read configuration file")]
	FailedToReadConfigurationFile(std::io::Error),

	#[error("malformed configuration file: {0}")]
	MalformatedConfigurationFile(#[from] toml::de::Error),

	#[error("failed to serialize configuration: {0}")]
	FailedToSerializeConfiguration(#[from] toml::ser::Error),

	#[error("failed to save configuration file")]
	FailedToSaveConfigurationFile(std::io::Error),

	#[error("failed to read from stdin")]
	FailedToReadStdIn(std::io::Error),

	#[error("no input file or stdin specified")]
	NoInputFileOrStdInSpecified,

	#[error("input file and stdin specified")]
	InputFileAndStdInSpecified,

	#[error("failed to read input file")]
	FailedToReadInputFile(std::io::Error),

	#[error("output file and stdout specified")]
	OutputFileAndStdOutSpecified,

	#[error("failed to create output file")]
	FailedToCreateOutputFile(std::io::Error),

	#[error("failed to create temporary file")]
	FailedToCreateTemporaryFile(std::io::Error),

	#[error("failed to get temporary file path")]
	FailedToGetTemporaryFilePath(std::io::Error),

	#[error("failed to replace input file")]
	FailedToReplaceInputFile(std::io::Error),
}

pub fn format(command: &Command) -> Result<(), FormatError> {
	let mut settings = Settings::default();

	if command.use_configuration {
		let path = match (&command.file_location, &command.path) {
			(Some(path), _) => {
				if path.extension().is_some() {
					path.parent().ok_or(FormatError::FailedToGetProjectFolder)?
						.to_owned()
				} else {
					path.to_owned()
				}
			}
			(_, Some(path)) => path.to_owned(),
			_ => std::env::current_dir()
				.map_err(FormatError::FailedToGetWorkingDirectory)?
				.to_owned(),
		};
		let mut path = path.as_path();
		let file = loop {
			let mut file = PathBuf::from(path);
			file.push(CONFIG_NAME);
			if file.is_file() {
				break file;
			}
			path = path.parent().ok_or(FormatError::NoConfigurationFile)?;
		};
		settings.overwrite(&file)?;
	}

	if command.save_configuration {
		std::fs::write(CONFIG_NAME, toml::to_string_pretty(&settings)?)
			.map_err(FormatError::FailedToSaveConfigurationFile)?;
		return Ok(());
	}

	let (input_data, input_name) = match (&command.path, command.use_std_in) {
		(Some(_), true) => return Err(FormatError::InputFileAndStdInSpecified),
		(Some(path), false) => {
			let input_data = std::fs::read_to_string(path).map_err(FormatError::FailedToReadInputFile)?;
			(input_data, path.display().to_string())
		}
		(None, true) => {
			let mut data = String::new();
			std::io::stdin()
				.read_to_string(&mut data)
				.map_err(FormatError::FailedToReadStdIn)?;
			(data, "stdin".into())
		}
		(None, false) => return Err(FormatError::NoInputFileOrStdInSpecified),
	};
	let root = ast::SourceFile::parse(&input_data).syntax_node();
	if command.use_std_out.not() {
		println!("DEVELOP: {:#?}", root);
	}

	match (&command.output, command.use_std_out) {
		(Some(_), true) => return Err(FormatError::OutputFileAndStdOutSpecified),
		(Some(out), false) => {
			let file = File::create(out).map_err(FormatError::FailedToCreateOutputFile)?;
			let mut target = BufWriter::new(file);
			format_node(root, &settings, &mut target)?;
			drop(target);
		}
		(None, true) => {
			let mut target = BufWriter::new(std::io::stdout());
			format_node(root, &settings, &mut target)?;
			drop(target);
		}
		(None, false) => {
			let temp_path = format!("{}.tmp", input_name);
			let file = File::create(&temp_path).map_err(FormatError::FailedToCreateTemporaryFile)?;
			let mut target = BufWriter::new(file);
			format_node(root, &settings, &mut target)?;
			drop(target);

			std::fs::rename(temp_path, input_name)
				.map_err(FormatError::FailedToReplaceInputFile)?;
		},
	};
	Ok(())
}

pub fn format_node(
	node: SyntaxNode,
	settings: &Settings,
	target: &mut impl OutputTarget,
) -> Result<(), FormatError> {
	let mut output = Output::new(target);
	let state = State::new();
	logic::format(Node::Node(node), state, settings, &mut output);
	output.finish(state, settings);
	Ok(())
}

pub type Node = NodeOrToken<SyntaxNode, SyntaxToken>;

pub struct Children(Option<ra_ap_syntax::SyntaxElementChildren>);

pub trait NodeExtension {
	fn text(&self) -> &str;

	fn children(&self) -> Children;
}

impl NodeExtension for Node {
	fn text(&self) -> &str {
		match self {
			NodeOrToken::Node(_) => "",
			NodeOrToken::Token(it) => it.text(),
		}
	}

	fn children(&self) -> Children {
		match self {
			NodeOrToken::Node(it) => Children(Some(it.children_with_tokens())),
			NodeOrToken::Token(_) => Children(None),
		}
	}
}

impl Iterator for Children {
	type Item = Node;

	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			Some(v) => v.next(),
			None => None,
		}
	}
}
