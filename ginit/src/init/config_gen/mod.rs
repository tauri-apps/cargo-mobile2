mod domain_blacklist;

use self::domain_blacklist::DOMAIN_BLACKLIST;
use crate::{app_name, ios, templating::template_pack, util::prompt};
use colored::*;
use heck::{KebabCase as _, TitleCase as _};
use hyphenation::Load as _;
use into_result::{command::CommandError, IntoResult as _};
use std::{env, process::Command, str};

#[derive(Debug)]
enum DefaultDomainError {
    FailedToGetGitEmailAddr(CommandError),
    EmailAddrInvalidUtf8(str::Utf8Error),
    FailedToParseEmailAddr,
}

fn default_domain() -> Result<Option<String>, DefaultDomainError> {
    let bytes = Command::new("git")
        .args(&["config", "user.email"])
        .output()
        .into_result()
        .map_err(DefaultDomainError::FailedToGetGitEmailAddr)
        .map(|output| output.stdout)?;
    let email = str::from_utf8(&bytes).map_err(DefaultDomainError::EmailAddrInvalidUtf8)?;
    let domain = email
        .trim()
        .split('@')
        .last()
        .ok_or(DefaultDomainError::FailedToParseEmailAddr)?;
    Ok(if DOMAIN_BLACKLIST.contains(&domain) {
        None
    } else {
        Some(domain.to_owned())
    })
}

pub fn interactive_config_gen(bike: &bicycle::Bicycle) {
    let dictionary = hyphenation::Standard::from_embedded(hyphenation::Language::EnglishUS)
        .expect("Failed to load dictionary");
    let wrapper = textwrap::Wrapper::with_splitter(textwrap::termwidth(), dictionary);

    let cwd = env::current_dir().expect("Failed to get current working directory");
    let app_name = {
        let mut default_app_name =
            app_name::transliterate(&cwd.file_name().unwrap().to_str().unwrap().to_kebab_case());
        let mut app_name = None;
        while let None = app_name {
            let response = prompt::default(
                "App name",
                default_app_name.as_ref().map(|s| s.as_str()),
                None,
            )
            .expect("Failed to prompt for app name");
            match app_name::validate(response) {
                Ok(response) => {
                    app_name = Some(response);
                }
                Err(err) => {
                    println!(
                        "{}",
                        wrapper
                            .fill(&format!("Gosh, that's not a valid app name! {}", err))
                            .bright_magenta()
                    );
                    if let Some(suggested) = err.suggested() {
                        default_app_name = Some(suggested.to_owned());
                    }
                }
            }
        }
        app_name.unwrap()
    };
    let stylized = {
        let stylized = app_name.replace("-", " ").replace("_", " ").to_title_case();
        prompt::default("Stylized app name", Some(&stylized), None)
    }
    .expect("Failed to prompt for stylized app name");
    let domain = {
        let default_domain = default_domain().ok().and_then(std::convert::identity);
        let default_domain = default_domain
            .as_ref()
            .map(|domain| domain.as_str())
            .unwrap_or_else(|| "example.com");
        prompt::default("Domain", Some(default_domain), None)
    }
    .expect("Failed to prompt for domain");
    let team = {
        let teams = ios::teams::find_development_teams().expect("Failed to find development teams");
        let mut default_team = None;
        println!("Detected development teams:");
        for (index, team) in teams.iter().enumerate() {
            println!(
                "  [{}] {} ({})",
                index.to_string().green(),
                team.name,
                team.id.cyan(),
            );
            if teams.len() == 1 {
                default_team = Some("0");
            }
        }
        if teams.is_empty() {
            println!("  -- none --");
        }
        println!(
            "  Enter an {} for a team above, or enter a {} manually.",
            "index".green(),
            "team ID".cyan(),
        );
        let team_input =
            prompt::default("Apple development team", default_team, Some(Color::Green))
                .expect("Failed to prompt for development team");
        team_input
            .parse::<usize>()
            .ok()
            .and_then(|index| teams.get(index))
            .map(|team| team.id.clone())
            .unwrap_or_else(|| team_input)
    };
    bike.process(
        template_pack(None, "{{tool_name}}.toml.hbs").expect("missing config template"),
        &cwd,
        |map| {
            map.insert("app_name", &app_name);
            map.insert("stylized_app_name", &stylized);
            map.insert("domain", &domain);
            map.insert("development_team", &team);
        },
    )
    .expect("Failed to render config file");
}
