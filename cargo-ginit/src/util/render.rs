use clap::{App, AppSettings, Arg, SubCommand};
use ginit::{core::cli, plugin::Plugin};

#[derive(Debug)]
pub struct TargetInfo<'a> {
    targets: Vec<&'a str>,
    default: &'a str,
}

impl<'a> TargetInfo<'a> {
    fn get_ref<'b>(&'b self) -> TargetInfoRef<'a, 'b> {
        TargetInfoRef {
            targets: &self.targets,
            default: self.default,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TargetInfoRef<'a, 'b> {
    targets: &'b [&'a str],
    default: &'a str,
}

#[derive(Debug)]
pub struct CliInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub cli: &'a cli::Cli,
    pub target_info: Option<TargetInfo<'a>>,
}

impl<'a> CliInfo<'a> {
    pub fn new<State>(plugin: &'a Plugin<State>, cli: &'a cli::Cli) -> Self {
        Self {
            name: plugin.name(),
            description: plugin.description(),
            cli,
            target_info: cli.target_info.as_ref().map(|target_info| TargetInfo {
                targets: target_info
                    .targets
                    .iter()
                    .map(|target| target.as_str())
                    .collect(),
                default: &target_info.default,
            }),
        }
    }

    pub fn render(&'a self) -> App<'a, 'a> {
        let mut app = SubCommand::with_name(self.name)
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .about(self.description);
        let target_info = self
            .target_info
            .as_ref()
            .map(|target_info| target_info.get_ref());
        for (order, sub) in self.cli.commands.iter().enumerate() {
            app = app.subcommand(command(sub, order, target_info));
        }
        app
    }
}

fn command<'a>(
    command: &'a cli::Command,
    order: usize,
    target_info: Option<TargetInfoRef<'a, 'a>>,
) -> App<'a, 'a> {
    let mut sub = SubCommand::with_name(&command.name)
        .about(command.about.as_str())
        .display_order(order);
    if command.hidden {
        sub = sub.setting(AppSettings::Hidden);
    }
    for argument in &command.args {
        sub = sub.arg(arg(argument, target_info));
    }
    sub
}

fn arg<'a>(arg: &'a cli::Arg, target_info: Option<TargetInfoRef<'a, 'a>>) -> Arg<'a, 'a> {
    match arg {
        cli::Arg::Custom {
            name,
            required,
            index,
        } => {
            let mut arg = Arg::with_name(name).required(*required);
            if let Some(index) = index {
                arg = arg.index(*index);
            }
            arg
        }
        cli::Arg::FromUsage { usage } => Arg::from_usage(usage),
        cli::Arg::TargetList => {
            if let Some(target_info) = target_info {
                super::take_a_target_list(target_info.targets, target_info.default)
            } else {
                unimplemented!()
            }
        }
        cli::Arg::Release => Arg::from_usage("--release 'Build with release optimizations'"),
    }
}
