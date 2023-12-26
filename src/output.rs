use crate::state::State;
use super::settings::Settings;


#[derive(Clone, Copy)]
pub enum Whitespace {
	None,
	Space,
	Spaces (usize),
	LineBreak,
	LineBreaks (usize),
}


impl Default for Whitespace {
	fn default() -> Self {
		Self::None
	}
}


impl Whitespace {
	pub fn from_text(text: &str) -> Self {
		let linebreaks = text.chars().filter(| &c | c == '\n').count();
		match (linebreaks, text.len()) {
			(_, 0) => Self::None,
			(0, 1) => Self::Space,
			(0, l) => Self::Spaces(l),
			(1, _) => Self::LineBreak,
			(l, _) => Self::LineBreaks(l),
		}
	}
}


pub trait OutputTarget {
	fn emit(&mut self, data: &str, settings: &Settings);
}


impl <T: std::io::Write> OutputTarget for T {
	fn emit(&mut self, data: &str, _settings: &Settings) {
		self.write_all(data.as_bytes()).unwrap();
	}
}


pub struct Output <'a, Target: OutputTarget> {
	target: &'a mut Target,
}


impl <'a, Target: OutputTarget> Output<'a, Target> {
	pub fn new(target: &'a mut Target) -> Self {
		Self {
			target,
		}
	}


	fn emit_indentation(&mut self, state: &State, settings: &Settings) {
		match settings.indentation {
			0 => self.target.emit(&format!("{0:\t<1$}", "", state.indentation()), settings),
			amount => self.target.emit(
				&format!("{0: <1$}", "", state.indentation() * amount),
				settings,
			),
		}
	}


	pub fn whitespace(&mut self, whitespace: Whitespace, state: &State) {
		match whitespace {
			Whitespace::None => { }
			Whitespace::Space => self.target.emit(" ", &state.settings()),
			Whitespace::Spaces(amount) => {
				self.target.emit(&format!("{0: <1$}", "", amount), &state.settings());
			}
			Whitespace::LineBreak => {
				self.target.emit("\n", &state.settings());
				self.emit_indentation(state, &state.settings())
			}
			Whitespace::LineBreaks(amount) => {
				self.target.emit(&format!("{0:\n<1$}", "", amount), &state.settings());
				self.emit_indentation(state, &state.settings())
			},
		}
	}


	pub fn text(&mut self, text: &str, state: &State) {
		if text.is_empty() {
			return;
		}
		self.target.emit(text, &state.settings());
	}
}


// pub struct PositionCalculator {
// 	line: usize,
// 	column: usize,
// }

// impl PositionCalculator {
// 	pub fn new() -> Self {
// 		Self { line: 0, column: 0 }
// 	}

// 	pub fn reset(&mut self) {
// 		self.line = 0;
// 		self.column = 0;
// 	}
// }

// impl OutputTarget for PositionCalculator {
// 	fn emit(&mut self, data: &str, settings: &Settings) {
// 		for symbol in data.chars() {
// 			match symbol {
// 				'\t' => {
// 					let tab_size = settings.indentation.max(1);
// 					self.column += 1 + tab_size.overflowing_sub(self.column).0 % tab_size
// 				}
// 				'\n' => {
// 					self.line += 1;
// 					self.column = 1;
// 				}
// 				_ => self.column += 1,
// 			}
// 		}
// 	}
// }

// impl Output<'_, PositionCalculator> {
// 	pub fn position(&self) -> (usize, usize) {
// 		(self.target.line, self.target.column)
// 	}

// 	pub fn reset(&mut self) {
// 		self.target.reset();
// 		self.whitespace = Whitespace::None;
// 		self.priority = Priority::Guaranteed;
// 	}
// }
