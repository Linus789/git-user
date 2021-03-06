# git-user

A simple way to change the user of a git repository.
Under the hood it executes:
```
git config user.name <name>
git config user.email <email>
```

![](demo.gif)

## Installation
In order to install, just run the following command

```
cargo install --force git-user
```

This will install git-user in your `~/.cargo/bin` (or Windows: `%USERPROFILE%\.cargo\bin`).

Make sure to add the `~/.cargo/bin` (or Windows: `%USERPROFILE%\.cargo\bin`) directory to your PATH variable.

## Commands
### Interactive
```
# Apply a profile to set the user for the local git repository
git-user

# Apply a profile to set the user for the local git repository
git-user apply

# Add a new user profile
git-user add

# Change a value in a profile (e. g. the email)
git-user set

# Remove a user profile
git-user remove
```

### CLI
```
# Apply a profile to set the user for the local git repository
git-user apply <profile>

# Add a new user profile
git-user add <profile> <name> <email>

# Add a new user profile
# Email defaults to: <name>@users.noreply.github.com
git-user add <profile> <name>

# Add a new user profile
# Name defaults to: <profile>
# Email defaults to: <profile>@users.noreply.github.com
git-user add <profile>

# Remove a user profile
git-user remove <profile>

# Rename a profile
git-user set profile <profile-name> <new-profile-name>

# Change the name of a profile
git-user set name <profile> <name>

# Change the email of a profile
git-user set email <profile> <email>

# Removes all directories and files ever created by this application
git-user reset

# Show the current user of the local git repository
git-user current

# List all profiles
git-user list

# Print the file path where the profiles are stored and its contents
git-user file

# Help menu
git-user -h
```

## Help menu
```
git-user 0.1.2
Linus789

USAGE:
    git-user [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add           Add a new user profile
    apply         Apply a profile to set the user for the local git repository
    current       Show the current user of the local git repository
    file          Print the file path where the profiles are stored and its contents
    help          Prints this message or the help of the given subcommand(s)
    list          List all profiles
    remove        Remove a user profile
    reset         Removes all directories and files ever created by this application
    set           Set a new value in a profile (e. g. to change the email)
```