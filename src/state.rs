use std::ops::Not;
use crate::settings::Settings;


#[derive(Debug)]
pub struct State {
	settings: Settings,
	indentation: usize,
	chained: bool,
}


pub struct Save {
	indetation: usize,
	chained: bool,
}


impl State {
	pub fn new(settings: Settings) -> Self {
		Self {
			settings,
			indentation: 0,
			chained: false,
		}
	}


	pub fn indent(&mut self) {
		self.indentation += 1;
		self.chained = false;
	}


	pub fn enter_scope(&mut self) {
		self.chained = false
	}


	pub fn dedent(&mut self) {
		self.indentation -= 1
	}


	pub fn start_chain(&mut self) {
		if self.chained {
			return;
		}
		self.chained = true;
		self.indentation += 1;
	}


	pub fn in_chain(&self) -> bool {
		self.chained
	}


	pub fn exit_chain(&mut self) {
		if self.chained.not() {
			return;
		}
		self.chained = false;
		self.indentation -= 1;
	}


	pub fn settings(&self) -> &Settings {
		&self.settings
	}


	pub fn indentation(&self) -> usize {
		self.indentation
	}


	pub fn save(&self) -> Save {
		Save {
			indetation: self.indentation,
			chained: self.chained,
		}
	}


	pub fn restore(&mut self, save: Save) {
		self.indentation = save.indetation;
		self.chained = save.chained;
	}
}
