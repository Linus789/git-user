mod cli;

use std::fs::OpenOptions;
use std::io::Read;

use toml::Value;

struct Item;

impl Item {
    const NAME: &'static str = "name";
    const EMAIL: &'static str = "email";
}

struct Git;

impl Git {
    const ATTR_NAME: &'static str = "user.name";
    const ATTR_EMAIL: &'static str = "user.email";
}

fn main() -> std::io::Result<()> {
    let data_dir = match dirs::data_dir() {
        None => {
            eprintln!("ERROR: Could not find data directoy");
            std::process::exit(2);
        }
        Some(x) => x,
    };

    let git_user_dir_path = data_dir.join("git-user");
    let git_user_file_path = git_user_dir_path.join("profiles");

    // create directory if non-existent
    std::fs::create_dir_all(&git_user_dir_path)?;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&git_user_file_path)?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    let file_items = toml::from_str::<toml::Value>(&buffer)?;

    if !is_valid_data(&file_items) {
        if let Some(path) = git_user_file_path.to_str() {
            eprintln!("ERROR: Data file ({}) corrupted", path);
        } else {
            eprintln!("ERROR: Data file corrupted");
        }

        std::process::exit(5);
    }

    // So the cursor will not disappear
    // when you are prompted via dialoguer and press ctrl-c
    ctrlc::set_handler(move || {
        let term = console::Term::stderr();
        let _ = term.show_cursor();
        std::process::exit(130);
    })
    .expect("Error setting Ctrl-C handler");

    cli::execute(&git_user_file_path, &buffer, file_items.as_table().unwrap().clone())?;

    Ok(())
}

fn is_valid_data(value: &Value) -> bool {
    let table = match value.as_table() {
        None => return false,
        Some(x) => x,
    };

    for value in table.values() {
        let item = match value.as_table() {
            None => return false,
            Some(x) => x,
        };

        if !item.contains_key(Item::NAME) || !item.contains_key(Item::EMAIL) {
            return false;
        }

        if !item.get(Item::NAME).unwrap().is_str() || !item.get(Item::EMAIL).unwrap().is_str() {
            return false;
        }
    }

    true
}
