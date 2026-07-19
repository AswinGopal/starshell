use indexmap::IndexMap;
use serde::{self, Deserialize, Serialize};

pub mod character;
pub mod custom;
pub mod directory;
pub mod git_branch;
pub mod git_commit;
pub mod git_state;
pub mod git_status;
pub mod line_break;
pub mod os;
pub mod python;
mod starship_root;
pub mod time;
pub mod username;

pub use starship_root::*;

#[derive(Serialize, Deserialize, Clone, Default)]
#[cfg_attr(
    feature = "config-schema",
    derive(schemars::JsonSchema),
    schemars(deny_unknown_fields)
)]
#[serde(default)]
pub struct FullConfig<'a> {
    // Meta
    #[serde(rename = "$schema")]
    schema: String,
    // Root config
    #[serde(flatten)]
    root: StarshipRootConfig,
    // modules
    #[serde(borrow)]
    character: character::CharacterConfig<'a>,
    #[serde(borrow)]
    directory: directory::DirectoryConfig<'a>,
    #[serde(borrow)]
    git_branch: git_branch::GitBranchConfig<'a>,
    #[serde(borrow)]
    git_commit: git_commit::GitCommitConfig<'a>,
    #[serde(borrow)]
    git_state: git_state::GitStateConfig<'a>,
    #[serde(borrow)]
    git_status: git_status::GitStatusConfig<'a>,
    line_break: line_break::LineBreakConfig,
    #[serde(borrow)]
    os: os::OSConfig<'a>,
    #[serde(borrow)]
    python: python::PythonConfig<'a>,
    #[serde(borrow)]
    time: time::TimeConfig<'a>,
    #[serde(borrow)]
    username: username::UsernameConfig<'a>,
    #[serde(borrow)]
    custom: IndexMap<String, custom::CustomConfig<'a>>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::module::ALL_MODULES;
    use toml::value::Value;

    #[test]
    fn test_all_modules_in_full_config() {
        let full_cfg = Value::try_from(FullConfig::default()).unwrap();
        let cfg_table = full_cfg.as_table().unwrap();
        for module in ALL_MODULES {
            assert!(cfg_table.contains_key(*module));
        }
    }
}
