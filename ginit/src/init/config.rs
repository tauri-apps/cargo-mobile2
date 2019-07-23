use crate::ios;
use colored::*;
use inflector::Inflector;
use std::{
    env,
    io::{self, Write},
};

pub fn prompt(
    label: &str,
    default: Option<&str>,
    default_color: Option<Color>,
) -> io::Result<String> {
    let mut input = String::new();
    if let Some(default) = default {
        if let Some(default_color) = default_color {
            print!("{} ({}): ", label, default.color(default_color));
        } else {
            print!("{} ({}): ", label, default);
        }
    } else {
        print!("{}: ", label);
    }
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    input = input.trim().to_owned();
    if input.is_empty() && default.is_some() {
        input = default.unwrap().to_owned();
    }
    Ok(input)
}

pub fn interactive_config_gen(bike: &bicycle::Bicycle) {
    let cwd = env::current_dir().expect("Failed to get current working directory");
    let dir_name = cwd.file_name().unwrap().to_str().unwrap();
    let app_name = prompt("App name", Some(dir_name), None).expect("Failed to prompt for app name");
    let stylized = app_name.replace("-", " ").replace("_", " ").to_title_case();
    let stylized = prompt("Stylized app name", Some(&stylized), None)
        .expect("Failed to prompt for stylized app name");
    let domain = prompt("Domain", Some("example.com"), None).expect("Failed to prompt for domain");
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
    let team_input = prompt("Apple development team", default_team, Some(Color::Green))
        .expect("Failed to prompt for development team");
    let team = team_input
        .parse::<usize>()
        .ok()
        .and_then(|index| teams.get(index))
        .map(|team| team.id.clone())
        .unwrap_or_else(|| team_input);
    bike.process(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/{{tool_name}}.toml.hbs"
        ),
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
