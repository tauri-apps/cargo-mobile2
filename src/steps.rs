use std::fmt::{self, Display};

#[derive(Debug)]
pub struct NotRegistered {
    step: String,
    registered_steps: &'static [&'static str],
}

impl Display for NotRegistered {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Step {:?} hasn't been registered (currently registered steps: {:?})",
            self.step, self.registered_steps
        )
    }
}

#[derive(Debug)]
pub struct Registry {
    steps: &'static [&'static str],
}

impl Registry {
    pub fn new(steps: &'static [&'static str]) -> Self {
        Self { steps }
    }

    fn flag(&self, step: impl AsRef<str>) -> Result<u32, NotRegistered> {
        if let Some(position) = self.steps.iter().position(|s| *s == step.as_ref()) {
            Ok(2u32.pow(position as u32))
        } else {
            Err(NotRegistered {
                step: step.as_ref().to_string(),
                registered_steps: self.steps,
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

    pub fn parse(registry: &'a Registry, steps: &[impl AsRef<str>]) -> Result<Self, NotRegistered> {
        let mut this = Self::new_all_unset(registry);
        for step in steps {
            this.set(step)?;
        }
        Ok(this)
    }

    pub fn bits(&self) -> u32 {
        self.bits
    }

    pub fn set(&mut self, step: impl AsRef<str>) -> Result<(), NotRegistered> {
        self.registry.flag(step).map(|flag| self.bits |= flag)
    }

    // pub fn unset(&mut self, step: impl AsRef<str>) -> Result<(), NotRegistered> {
    //     self.registry.flag(step).map(|flag| self.bits ^= flag)
    // }

    pub fn try_is_set(&self, step: impl AsRef<str>) -> Result<bool, NotRegistered> {
        self.registry
            .flag(step)
            .map(|flag| self.bits & flag == flag)
    }

    pub fn is_set(&self, step: impl AsRef<str>) -> bool {
        let step = step.as_ref();
        let result = self.try_is_set(step);
        match result {
            Ok(val) => val,
            Err(_) => {
                let msg = format!("developer error: {:?} step not registered", step);
                result.expect(&msg)
            }
        }
    }
}
