use clap::{Arg, ArgMatches};
use ginit::{opts::Profile, target::TargetTrait};

pub fn take_a_list<'a, 'b>(arg: Arg<'a, 'b>, values: &'a [&'a str]) -> Arg<'a, 'b> {
    arg.possible_values(values)
        .multiple(true)
        .value_delimiter(" ")
}

pub fn take_a_target_list<'a, 'b, T: TargetTrait<'a>>(targets: &'a [&'a str]) -> Arg<'a, 'b> {
    take_a_list(Arg::with_name("TARGETS"), targets).default_value(T::DEFAULT_KEY)
}

pub fn parse_targets(matches: &ArgMatches<'_>) -> Vec<String> {
    matches
        .values_of("TARGETS")
        .map(|vals| vals.map(Into::into).collect())
        .unwrap_or_default()
}

pub fn parse_profile(matches: &ArgMatches<'_>) -> Profile {
    if matches.is_present("release") {
        Profile::Release
    } else {
        Profile::Debug
    }
}

#[macro_export]
macro_rules! detect_device {
    ($func:path, $name:ident) => {
        fn detect_device<'a>(env: &'_ Env) -> Result<Device<'a>, Error> {
            let device_list = $func(env).map_err(Error::DeviceDetectionFailed)?;
            if device_list.len() > 0 {
                let index = if device_list.len() > 1 {
                    prompt::list(
                        concat!("Detected ", stringify!($name), " devices"),
                        device_list.iter(),
                        "device",
                        None,
                        "Device",
                    )
                    .map_err(Error::DevicePromptFailed)?
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
                Err(Error::NoDevicesDetected)
            }
        }
    };
}
