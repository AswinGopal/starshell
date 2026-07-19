// While adding out new module add out module to src/module.rs ALL_MODULES const array also.
mod character;
pub mod custom;
mod directory;
mod env_var;
mod git_branch;
mod git_commit;
mod git_state;
pub mod git_status;
mod line_break;
mod os;
mod python;
mod time;
mod username;
mod utils;

use crate::config::ModuleConfig;
use crate::context::{Context, Detected, Shell};
use crate::module::Module;
use std::time::Instant;

pub fn handle<'a>(module: &str, context: &'a Context) -> Option<Module<'a>> {
    let start: Instant = Instant::now();
    let mut m: Option<Module> = {
        match module {
            // Keep these ordered alphabetically.
            // Default ordering is handled in configs/starship_root.rs
            "character" => character::module(context),
            "directory" => directory::module(context),
            "git_branch" => git_branch::module(context),
            "git_commit" => git_commit::module(context),
            "git_state" => git_state::module(context),
            "git_status" => git_status::module(context),
            "line_break" => line_break::module(context),
            "os" => os::module(context),
            "python" => python::module(context),
            "time" => time::module(context),
            "username" => username::module(context),
            env if env.starts_with("env_var.") => {
                env_var::module(env.strip_prefix("env_var."), context)
            }
            custom if custom.starts_with("custom.") => {
                // SAFETY: We just checked that the module starts with "custom."
                custom::module(custom.strip_prefix("custom.").unwrap(), context)
            }
            _ => {
                eprintln!(
                    "Error: Unknown module {module}. Use starship module --list to list out all supported modules."
                );
                None
            }
        }
    };

    let elapsed = start.elapsed();
    log::trace!("Took {elapsed:?} to compute module {module:?}");
    if elapsed.as_millis() >= 1 {
        // If we take less than 1ms to compute a None, then we will not return a module at all
        // if we have a module: default duration is 0 so no need to change it
        // if we took more than 1ms we want to report that and so--in case we have None currently--
        // need to create an empty module just to hold the duration for that case
        m.get_or_insert_with(|| context.new_module(module)).duration = elapsed;
    }
    m
}

pub fn description(module: &str) -> &'static str {
    match module {
        "character" => {
            "A character (usually an arrow) beside where the text is entered in your terminal"
        }
        "directory" => "The current working directory",
        "git_branch" => "The active branch of the repo in your current directory",
        "git_commit" => "The active commit (and tag if any) of the repo in your current directory",
        "git_state" => "The current git operation, and it's progress",
        "git_status" => "Symbol representing the state of the repo",
        "line_break" => "Separates the prompt into two lines",
        "os" => "The current operating system",
        "python" => "The currently installed version of Python",
        "time" => "The current local time",
        "username" => "The active user's username",
        _ => "<no description>",
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::module::ALL_MODULES;

    #[test]
    fn all_modules_have_description() {
        for module in ALL_MODULES {
            println!("Checking if {module:?} has a description");
            assert_ne!(description(module), "<no description>");
        }
    }
}
