use ginit::{
    core::{
        exports::clap::{App, Arg, ArgMatches},
        opts,
        util::cli,
    },
    init,
    plugin::Map as PluginMap,
};

pub fn app<'a, 'b>(steps: &'a [&'a str]) -> App<'a, 'b> {
    cli::take_init_subcommand()
        .arg_from_usage("--open 'Open in VS Code'")
        .arg(cli::take_a_list(
            Arg::with_name("only")
                .long("only")
                .help("Only do some steps")
                .value_name("STEPS"),
            steps,
        ))
        .arg(cli::take_a_list(
            Arg::with_name("skip")
                .long("skip")
                .help("Skip some steps")
                .value_name("STEPS"),
            steps,
        ))
}

#[derive(Debug)]
pub struct Command {
    clobbering: opts::Clobbering,
    open_in: opts::OpenIn,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
}

impl cli::CommandTrait for Command {
    fn parse(matches: &ArgMatches<'_>) -> Self {
        let clobbering = cli::parse_clobbering(&matches);
        let open_in = if matches.is_present("open") {
            opts::OpenIn::Editor
        } else {
            opts::OpenIn::Nothing
        };
        let only = matches.args.get("only").map(|only| {
            only.vals
                .iter()
                .map(|step| step.to_string_lossy().into_owned())
                .collect()
        });
        let skip = matches.args.get("skip").map(|skip| {
            skip.vals
                .iter()
                .map(|step| step.to_string_lossy().into_owned())
                .collect()
        });
        Self {
            clobbering,
            open_in,
            only,
            skip,
        }
    }
}

pub fn exec(
    cli::Input {
        noise_level,
        interactivity,
        command:
            Command {
                clobbering,
                open_in,
                only,
                skip,
            },
    }: cli::Input<Command>,
    plugins: &PluginMap,
) -> Result<(), init::Error> {
    init::init(
        plugins,
        noise_level,
        interactivity,
        clobbering,
        open_in,
        only.as_ref().map(|only| only.as_slice()),
        skip.as_ref().map(|skip| skip.as_slice()),
    )
}
