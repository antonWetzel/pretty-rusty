#[derive(Clone, Copy, Debug)]
pub struct State {
    pub indentation: usize,
    pub scope: Scope,
}

impl State {
    pub fn new() -> Self {
        Self {
            indentation: 0,
            scope: Scope::Default,
        }
    }

    pub fn indent(&mut self) {
        self.indentation += 1;
    }

    pub fn dedent(&mut self) {
        self.indentation -= 1
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    Default,
    CompactList,
    PaddedList,
    MultilinList,
}
