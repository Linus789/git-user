use std::borrow::Cow;
use std::path::Path;
use std::process::{Command, Stdio};

use clap::{AppSettings, Clap};
use toml::value::Table;

use crate::cli::RootSubcommand::*;
use crate::cli::SetSubcommand::*;
use crate::{Git, Item};

pub fn execute(path: &Path, file_contents: &str, mut table: Table) -> std::io::Result<()> {
    let opts = Opts::parse();

    // In some cases git has to be installed
    let (git_needed, git_dir_needed) = if let Some(subcmd) = &opts.subcmd {
        match &subcmd {
            Apply(_) => (true, true),
            Current => (true, true),
            _ => (false, false),
        }
    } else {
        (true, true)
    };

    if git_needed {
        // git version
        let git_version = Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !git_version.success() {
            eprintln!("ERROR: git not installed");
            std::process::exit(git_version.code().unwrap_or(1));
        }
    }

    if git_dir_needed {
        // git status
        let git_status = Command::new("git")
            .arg("status")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !git_status.success() {
            eprintln!("ERROR: not inside a git directory");
            std::process::exit(2);
        }
    }

    // No argument passed? => show prompt to apply a profile
    if opts.subcmd.is_none() {
        let profile = prompt_select_profile(&table, true)?;
        apply(profile.name, profile.email)?;
        return Ok(());
    }

    // Process input
    match opts.subcmd.unwrap() {
        Apply(profile) => {
            let profile = if let Some(profile) = profile.profile {
                Cow::Owned(profile)
            } else {
                Cow::Borrowed(prompt_select_profile(&table, true)?.profile)
            };

            if !ProfileRequirement::Existent.check_and_print(&table, &profile) {
                std::process::exit(1);
            }

            let name = get_name(&table, &profile);
            let email = get_email(&table, &profile);

            apply(name, email)?;
        }
        Add(add) => {
            let default_email = |name: &str| -> String { format!("{}@users.noreply.github.com", name) };

            let (profile, name, email) = if let Some(profile) = add.profile {
                if !ProfileRequirement::NonExistent.check_and_print(&table, &profile) {
                    std::process::exit(1);
                }

                let name = add.name.unwrap_or_else(|| profile.clone());
                let email = add.email.unwrap_or_else(|| default_email(&name));
                (profile, name, email)
            } else {
                let profile = prompt_input("Profile", None);

                if !ProfileRequirement::NonExistent.check_and_print(&table, &profile) {
                    std::process::exit(1);
                }

                let name = prompt_input("Name", Some(profile.to_string()));
                let email = prompt_input("Email", Some(default_email(&name)));
                (profile, name, email)
            };

            let mut item_table = Table::with_capacity(2);
            item_table.insert(Item::NAME.to_string(), toml::Value::String(name));
            item_table.insert(Item::EMAIL.to_string(), toml::Value::String(email));

            table.insert(profile, toml::Value::Table(item_table));
            write_toml(path, &table)?;
        }
        Remove(profile) => {
            let profile = if let Some(profile) = profile.profile {
                profile
            } else {
                prompt_select_profile(&table, false)?.profile.to_string()
            };

            if !ProfileRequirement::Existent.check_and_print(&table, &profile) {
                std::process::exit(1);
            }

            table.remove(&profile);
            write_toml(path, &table)?;
        }
        Reset => {
            std::fs::write(&path, "")?;
        }
        Set(subcmd) => {
            match subcmd.subcmd {
                Profile(set_profile) => {
                    if !ProfileRequirement::Existent.check_and_print(&table, &set_profile.profile_name) {
                        std::process::exit(1);
                    }

                    if !ProfileRequirement::NonExistent.check_and_print(&table, &set_profile.new_profile_name) {
                        std::process::exit(1);
                    }

                    let val = table.remove(&set_profile.profile_name).unwrap();
                    table.insert(set_profile.new_profile_name, val);
                }
                Name(set_name) => {
                    if !ProfileRequirement::Existent.check_and_print(&table, &set_name.profile) {
                        std::process::exit(1);
                    }

                    table
                        .get_mut(&set_name.profile)
                        .unwrap()
                        .as_table_mut()
                        .unwrap()
                        .insert(Item::NAME.to_string(), toml::Value::String(set_name.name));
                }
                Email(set_email) => {
                    if !ProfileRequirement::Existent.check_and_print(&table, &set_email.profile) {
                        std::process::exit(1);
                    }

                    table
                        .get_mut(&set_email.profile)
                        .unwrap()
                        .as_table_mut()
                        .unwrap()
                        .insert(Item::EMAIL.to_string(), toml::Value::String(set_email.email));
                }
            }

            write_toml(path, &table)?;
        }
        File => {
            if let Some(path_str) = path.to_str() {
                println!(">>> {}", path_str);
            }

            println!("{}", file_contents);
        }
        Current => {
            let check = |git_result: &GitResult, attr: &str| {
                println!("> git config {}", attr);

                if !git_result.success {
                    println!("WARNING: {} not set", attr);
                } else {
                    println!("{}", git_result.output);
                }
            };

            let git_name = get_git_name()?;
            check(&git_name, Git::ATTR_NAME);

            println!();

            let git_email = get_git_email()?;
            check(&git_email, Git::ATTR_EMAIL);
        }
        List => {
            for (index, (profile, _)) in table.iter().enumerate() {
                if index != 0 {
                    println!();
                }

                println!("Profile: {}", profile);
                println!("Name: {}", get_name(&table, profile));
                println!("Email: {}", get_email(&table, profile));
            }
        }
    }

    Ok(())
}

/// Returns selected profile
fn prompt_select_profile(table: &Table, default_current: bool) -> std::io::Result<ProfileInfo> {
    let mut profiles: Vec<ProfileInfo> = Vec::with_capacity(table.len());
    let mut display: Vec<String> = Vec::with_capacity(table.len());

    let curr_name = if default_current {
        Some(get_git_name()?.output)
    } else {
        None
    };

    let curr_email = if default_current {
        Some(get_git_email()?.output)
    } else {
        None
    };

    let mut default_index: usize = 0;

    for (index, (profile, _)) in table.iter().enumerate() {
        let name = get_name(table, profile);
        let email = get_email(table, profile);

        profiles.push(ProfileInfo { profile, name, email });

        display.push(format!("{} : {}", name, email));

        if default_current && name == curr_name.as_deref().unwrap() && email == curr_email.as_deref().unwrap() {
            default_index = index;
        }
    }

    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Select a git user")
        .default(default_index)
        .items(&display[..])
        .interact();

    // In case ctrl-c is used to exit, do not print an error
    handle_prompt_error(&selection);

    Ok(profiles.remove(selection.unwrap()))
}

fn prompt_input(title: &str, default: Option<String>) -> String {
    let theme = dialoguer::theme::ColorfulTheme::default();
    let mut input: dialoguer::Input<String> = dialoguer::Input::with_theme(&theme);

    input.with_prompt(title);

    if let Some(default) = default {
        input.default(default);
    }

    let result = input.interact_text();

    // In case ctrl-c is used to exit, do not print an error
    handle_prompt_error(&result);

    result.unwrap()
}

fn handle_prompt_error<T>(error: &std::io::Result<T>) {
    if let Err(error) = error {
        if error.kind() != std::io::ErrorKind::Interrupted {
            println!("{}", error);
        } else {
            println!();
        }

        std::process::exit(130);
    }
}

/// Apply name and email to the local git repository
fn apply(name: &str, email: &str) -> std::io::Result<()> {
    let set_attr = |attr: &str, value: &str| -> std::io::Result<()> {
        let success = Command::new("git")
            .arg("config")
            .arg(attr)
            .arg(value)
            .output()?
            .status
            .success();

        if !success {
            eprintln!("ERROR: Failed to set {}", attr);
        }

        Ok(())
    };

    // git config user.name "Your Name"
    set_attr(Git::ATTR_NAME, name)?;

    // git config user.email "you@example.com"
    set_attr(Git::ATTR_EMAIL, email)?;

    Ok(())
}

fn get_name<'a>(root_table: &'a Table, profile: &str) -> &'a str {
    root_table
        .get(profile)
        .unwrap()
        .as_table()
        .unwrap()
        .get(Item::NAME)
        .unwrap()
        .as_str()
        .unwrap()
}

fn get_email<'a>(root_table: &'a Table, profile: &str) -> &'a str {
    root_table
        .get(profile)
        .unwrap()
        .as_table()
        .unwrap()
        .get(Item::EMAIL)
        .unwrap()
        .as_str()
        .unwrap()
}

struct ProfileInfo<'a> {
    profile: &'a str,
    name: &'a str,
    email: &'a str,
}

struct GitResult {
    output: String,
    success: bool,
}

enum ProfileRequirement {
    Existent,
    NonExistent,
}

impl ProfileRequirement {
    /// Returns true if the requirement is fulfilled, otherwise false
    fn check_and_print(&self, table: &Table, profile: &str) -> bool {
        let profile_exists = table.contains_key(profile);

        match &self {
            ProfileRequirement::Existent => {
                if !profile_exists {
                    eprintln!("ERROR: Profile '{}' could not be found", profile);
                    return false;
                }
            }
            ProfileRequirement::NonExistent => {
                if profile_exists {
                    eprintln!("ERROR: Profile '{}' already exists", profile);
                    return false;
                }
            }
        }

        true
    }
}

fn get_git_attr(attr: &str) -> std::io::Result<GitResult> {
    let output = Command::new("git").arg("config").arg(attr).output()?;
    let lines = String::from_utf8_lossy(&output.stdout);
    let name = lines.lines().next().unwrap_or("");

    Ok(GitResult {
        output: name.to_string(),
        success: output.status.success(),
    })
}

fn get_git_name() -> std::io::Result<GitResult> {
    get_git_attr(Git::ATTR_NAME)
}

fn get_git_email() -> std::io::Result<GitResult> {
    get_git_attr(Git::ATTR_EMAIL)
}

fn write_toml<T: serde::ser::Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let toml = toml::to_string(value).unwrap();
    std::fs::write(&path, toml)?;
    Ok(())
}

#[derive(Clap)]
#[clap(setting = AppSettings::VersionlessSubcommands)]
#[clap(version = "0.1.0", author = "Linus789")]
struct Opts {
    #[clap(subcommand)]
    subcmd: Option<RootSubcommand>,
}

#[derive(Clap)]
enum RootSubcommand {
    /// Apply a profile to set the user for the local git repository
    Apply(Profile),
    /// Add a new user profile
    Add(Add),
    /// Remove a user profile
    Remove(Profile),
    /// Remove all user profiles
    Reset,
    /// Set a new value in a profile (e. g. to change the email)
    Set(SetCommand),
    /// Print the file path where the profiles are stored and its contents
    File,
    /// Show the current user of the local git repository
    Current,
    /// List all profiles
    List,
}

#[derive(Clap)]
struct SetCommand {
    #[clap(subcommand)]
    subcmd: SetSubcommand,
}

#[derive(Clap)]
enum SetSubcommand {
    /// To change profile values, e. g. the email
    Profile(SetProfile),
    /// To change the name of a profile
    Name(SetName),
    /// To change the email of a profile
    Email(SetEmail),
}

#[derive(Clap)]
struct Profile {
    profile: Option<String>,
}

#[derive(Clap)]
struct Add {
    profile: Option<String>,
    name: Option<String>,
    email: Option<String>,
}

#[derive(Clap)]
struct SetProfile {
    profile_name: String,
    new_profile_name: String,
}

#[derive(Clap)]
struct SetName {
    profile: String,
    name: String,
}

#[derive(Clap)]
struct SetEmail {
    profile: String,
    email: String,
}
