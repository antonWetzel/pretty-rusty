#![doc = include_str!("../readme.md")] #![forbid(unsafe_code, clippy::unwrap_used)]


use std::{ path::PathBuf, io::{ Read, BufWriter }, ops::Not, fs::File };
use clap::Parser;
use pretty_rusty::{ Settings, format_node, ast };


const CONFIG_NAME: &str = "pretty-rusty.toml";


#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("Failed to get project folder")]
	FailedToGetProjectFolder,

	#[error("Failed to get working directory")]
	FailedToGetWorkingDirectory (std::io::Error),

	#[error("No configuration file")]
	NoConfigurationFile,

	#[error("Failed to read configuration file")]
	FailedToReadConfigurationFile (std::io::Error),

	#[error("malformed configuration file: {0}")]
	MalformatedConfigurationFile (#[from] toml::de::Error),

	#[error("failed to serialize configuration: {0}")]
	FailedToSerializeConfiguration (#[from] toml::ser::Error),

	#[error("failed to save configuration file")]
	FailedToSaveConfigurationFile (std::io::Error),

	#[error("failed to read from stdin")]
	FailedToReadStdIn (std::io::Error),

	#[error("no input file or stdin specified")]
	NoInputFileOrStdInSpecified,

	#[error("input file and stdin specified")]
	InputFileAndStdInSpecified,

	#[error("failed to read input file")]
	FailedToReadInputFile (std::io::Error),

	#[error("output file and stdout specified")]
	OutputFileAndStdOutSpecified,

	#[error("failed to create output file")]
	FailedToCreateOutputFile (std::io::Error),

	#[error("failed to create temporary file")]
	FailedToCreateTemporaryFile (std::io::Error),

	#[error("failed to get temporary file path")]
	FailedToGetTemporaryFilePath (std::io::Error),

	#[error("failed to replace input file")]
	FailedToReplaceInputFile (std::io::Error),
}


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


fn main() {
	match format(&Command::parse()) {
		Ok(()) => { }
		Err(err) => {
			eprintln!("{}", err);
			std::process::exit(1);
		},
	}
}


fn format(command: &Command) -> Result<(), Error> {
	let mut settings = Settings::default();

	if command.use_configuration {
		let path = match (&command.file_location, &command.path) {
			(Some(path), _) => {
				if path.extension().is_some() {
					path.parent().ok_or(Error::FailedToGetProjectFolder)?.to_owned()
				} else {
					path.to_owned()
				}
			}
			(_, Some(path)) => path.to_owned(),
			_ => std::env::current_dir().map_err(Error::FailedToGetWorkingDirectory)?.to_owned(),
		};
		let mut path = path.as_path();
		let file = loop {
			let mut file = PathBuf::from(path);
			file.push(CONFIG_NAME);
			if file.is_file() {
				break file;
			}
			path = path.parent().ok_or(Error::NoConfigurationFile)?;
		};
		let data = std::fs::read_to_string(&file).map_err(Error::FailedToReadConfigurationFile)?;
		settings.overwrite(&data)?;
	}

	if command.save_configuration {
		std::fs::write(CONFIG_NAME, toml::to_string_pretty(&settings)?)
			.map_err(Error::FailedToSaveConfigurationFile)?;
		return Ok(());
	}

	let (input_data, input_name) = match (&command.path, command.use_std_in) {
		(Some(_), true) => return Err(Error::InputFileAndStdInSpecified),
		(Some(path), false) => {
			let input_data = std::fs::read_to_string(path)
				.map_err(Error::FailedToReadInputFile)?;
			(input_data, path.display().to_string())
		}
		(None, true) => {
			let mut data = String::new();
			std::io::stdin().read_to_string(&mut data).map_err(Error::FailedToReadStdIn)?;
			(data, "stdin".into())
		}
		(None, false) => return Err(Error::NoInputFileOrStdInSpecified),
	};
	let root = ast::ast::SourceFile::parse(&input_data).syntax_node();
	if command.use_std_out.not() {
		println!("{:#?}", root);
	}

	match (&command.output, command.use_std_out) {
		(Some(_), true) => return Err(Error::OutputFileAndStdOutSpecified),
		(Some(out), false) => {
			let file = File::create(out).map_err(Error::FailedToCreateOutputFile)?;
			let mut target = BufWriter::new(file);
			format_node(root, settings, &mut target);
			drop(target);
		}
		(None, true) => {
			let mut target = BufWriter::new(std::io::stdout());
			format_node(root, settings, &mut target);
			drop(target);
		}
		(None, false) => {
			let temp_path = format!("{}.tmp", input_name);
			let file = File::create(&temp_path).map_err(Error::FailedToCreateTemporaryFile)?;
			let mut target = BufWriter::new(file);
			format_node(root, settings, &mut target);
			drop(target);

			std::fs::rename(temp_path, input_name).map_err(Error::FailedToReplaceInputFile)?;
		}
	};

	Ok(())
}
