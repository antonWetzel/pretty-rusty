#[derive( Debug)]
pub struct State {
	pub indentation: usize,
    pub chain: usize,
}

impl State {
	pub fn new() -> Self {
		Self {
			indentation: 0,
            chain: 0,
		}
	}

	pub fn indent(&mut self) {
		self.indentation += 1;
	}

	pub fn dedent(&mut self) {
		self.indentation -= 1
	}
}
