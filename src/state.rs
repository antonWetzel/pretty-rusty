#[derive(Clone, Copy)]
pub struct State {
    pub indentation: usize,
}

impl State {
    pub fn new() -> Self {
        Self { indentation: 0 }
    }
    pub fn indent(&mut self) {
        self.indentation += 1;
    }

    pub fn dedent(&mut self) {
        self.indentation -= 1
    }
}
