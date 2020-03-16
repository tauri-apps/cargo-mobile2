use std::fmt::{self, Display};

#[derive(Debug)]
pub struct StepNotRegistered {
    step: String,
    registered_steps: Vec<String>,
}

impl Display for StepNotRegistered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Step {:?} hasn't been registered (currently registered steps: {:?})",
            self.step, self.registered_steps
        )
    }
}

#[derive(Debug, Default)]
pub struct Registry {
    steps: Vec<String>,
}

impl Registry {
    pub fn register(&mut self, step: impl Into<String>) {
        self.steps.push(step.into());
    }

    fn flag(&self, step: impl AsRef<str>) -> Result<u32, StepNotRegistered> {
        if let Some(position) = self.steps.iter().position(|s| s == step.as_ref()) {
            Ok(2u32.pow(position as u32))
        } else {
            Err(StepNotRegistered {
                step: step.as_ref().to_string(),
                registered_steps: self.steps.clone(),
            })
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Steps<'a> {
    registry: &'a Registry,
    bits: u32,
}

impl<'a> Steps<'a> {
    pub fn from_bits(registry: &'a Registry, bits: u32) -> Self {
        Self { registry, bits }
    }

    pub fn new_all_set(registry: &'a Registry) -> Self {
        Self::from_bits(registry, u32::max_value())
    }

    pub fn new_all_unset(registry: &'a Registry) -> Self {
        Self::from_bits(registry, 0)
    }

    pub fn parse(
        registry: &'a Registry,
        steps: &[impl AsRef<str>],
    ) -> Result<Self, StepNotRegistered> {
        let mut this = Self::new_all_unset(registry);
        for step in steps {
            this.set(step)?;
        }
        Ok(this)
    }

    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn set(&mut self, step: impl AsRef<str>) -> Result<(), StepNotRegistered> {
        self.registry.flag(step).map(|flag| self.bits |= flag)
    }

    // pub fn unset(&mut self, step: impl AsRef<str>) -> Result<(), StepNotRegistered> {
    //     self.registry.flag(step).map(|flag| self.bits ^= flag)
    // }

    pub fn is_set(&self, step: impl AsRef<str>) -> Result<bool, StepNotRegistered> {
        self.registry
            .flag(step)
            .map(|flag| self.bits & flag == flag)
    }
}
