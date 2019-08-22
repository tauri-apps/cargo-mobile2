#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NoiseLevel {
    Polite,
    LoudAndProud,
    FranklyQuitePedantic,
}

impl Default for NoiseLevel {
    fn default() -> Self {
        NoiseLevel::Polite
    }
}

impl NoiseLevel {
    pub fn is_verbose(self) -> bool {
        self > NoiseLevel::Polite
    }
}
