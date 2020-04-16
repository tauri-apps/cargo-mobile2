use crate::util::cli::{Report, Reportable};
use std::{fmt::Debug, io};

#[derive(Debug)]
pub enum PromptErrorCause<T: Reportable> {
    DetectionFailed(T),
    PromptFailed(io::Error),
    NoneDetected,
}

#[derive(Debug)]
pub struct PromptError<T: Reportable> {
    name: &'static str,
    cause: PromptErrorCause<T>,
}

impl<T: Reportable> Reportable for PromptError<T> {
    fn report(&self) -> Report {
        match &self.cause {
            PromptErrorCause::DetectionFailed(err) => err.report(),
            PromptErrorCause::PromptFailed(err) => {
                Report::error(format!("Failed to prompt for {} device", self.name), err)
            }
            PromptErrorCause::NoneDetected => Report::error(
                format!("Failed to prompt for {} device", self.name),
                format!("No connected {} devices detected", self.name),
            ),
        }
    }
}

impl<T: Reportable> PromptError<T> {
    pub fn new(name: &'static str, cause: PromptErrorCause<T>) -> Self {
        Self { name, cause }
    }

    pub fn detection_failed(name: &'static str, err: T) -> Self {
        Self::new(name, PromptErrorCause::DetectionFailed(err))
    }

    pub fn prompt_failed(name: &'static str, err: io::Error) -> Self {
        Self::new(name, PromptErrorCause::PromptFailed(err))
    }

    pub fn none_detected(name: &'static str) -> Self {
        Self::new(name, PromptErrorCause::NoneDetected)
    }
}

#[macro_export]
macro_rules! define_device_prompt {
    ($func:path, $e:ty, $name:ident) => {
        fn device_prompt<'a>(env: &'_ Env) -> Result<Device<'a>, $crate::device::PromptError<$e>> {
            let device_list = $func(env).map_err(|cause| {
                $crate::device::PromptError::detection_failed(stringify!($name), cause)
            })?;
            if device_list.len() > 0 {
                let index = if device_list.len() > 1 {
                    prompt::list(
                        concat!("Detected ", stringify!($name), " devices"),
                        device_list.iter(),
                        "device",
                        None,
                        "Device",
                    )
                    .map_err(|cause| {
                        $crate::device::PromptError::prompt_failed(stringify!($name), cause)
                    })?
                } else {
                    0
                };
                let device = device_list.into_iter().nth(index).unwrap();
                println!(
                    "Detected connected device: {} with target {:?}",
                    device,
                    device.target().triple,
                );
                Ok(device)
            } else {
                Err($crate::device::PromptError::none_detected(stringify!(
                    $name
                )))
            }
        }
    };
}
