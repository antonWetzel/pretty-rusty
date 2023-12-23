#![doc = include_str!("../readme.md")]
#![forbid(unsafe_code, clippy::unwrap_used)]

use clap::Parser;
use pretty_rusty::{ format, Command };

fn main() {
	match format(&Command::parse()) {
		Ok(()) => { }
		Err(err) => {
			eprintln!("{}", err);
			std::process::exit(1);
		},
	}
}
