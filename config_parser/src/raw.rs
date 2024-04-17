use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DeserializeFromStr, DisplayFromStr, SerializeDisplay};

#[derive(Debug)]
enum Variable<'a> {
    Env(&'a str),
}

trait ReplaceVariables {
    const VARIABLE_PREFIX: &'static str = "${{";
    const VARIABLE_SUFFIX: &'static str = "}}";

    fn is_variable(value: &str) -> bool {
        Self::get_inner(value).is_some()
    }

    fn get_inner<'a>(value: &'a str) -> Option<Variable<'a>> {
        value
            .strip_prefix(Self::VARIABLE_PREFIX)
            .map(|item| item.strip_suffix(Self::VARIABLE_SUFFIX))
            .flatten()
            .map(|item| item.trim())
            .map(|item| {
                if let Some(env_key) = item.strip_prefix("env.") {
                    Some(Variable::Env(env_key))
                } else {
                    todo!("Unknown variable: {:?}", item);
                }
            })
            .flatten()
    }

    fn replace(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, SerializeDisplay, DeserializeFromStr)]
pub enum ConfigVersion {
    V1_0Beta,
}

impl Display for ConfigVersion {
    // TODO maybe there is some way of a macro to derive Display and FromStr accordingly?
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigVersion::V1_0Beta => write!(f, "1.0-beta"),
        }
    }
}

impl FromStr for ConfigVersion {
    // TODO maybe there is some way of a macro to derive Display and FromStr accordingly?
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        match s {
            "1.0-beta" => Ok(ConfigVersion::V1_0Beta),
            _ => bail!("Unknown version: {}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub expose: u16,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub uses: String,
    pub name: Option<String>,
    #[serde(default)]
    pub with: HashMap<String, String>, // TODO maybe make the value type generic over sth
    #[serde(default)]
    pub arguments: HashMap<String, String>,
}

impl ReplaceVariables for Step {
    fn replace(&mut self) -> Result<()> {
        if Self::is_variable(&self.uses) {
            todo!("uses")
        }

        if let Some(name) = &mut self.name {
            if Self::is_variable(&name) {
                todo!("name")
            }
        }

        self.with.values_mut().for_each(|item| {
            if Self::is_variable(&item) {
                todo!("with")
            }
        });

        for argument in self.arguments.values_mut() {
            if let Some(inner_variable) = Self::get_inner(&argument) {
                let replace_with = match inner_variable {
                    Variable::Env(env_key) => std::env::var(env_key).with_context(|| {
                        format!(
                            "Could not find an environment variable with the name: '{:?}'",
                            env_key
                        )
                    })?,
                };

                *argument = replace_with;
            }
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    #[serde_as(as = "DisplayFromStr")]
    pub period: Schedule, // TODO the struct `Schedule` is really large, maybe box or rc/arc it?
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub path: String,
    pub pipeline: Vec<Step>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub version: ConfigVersion,
    pub config: Config,
    pub health_check: Option<HealthCheck>,
    pub route: Route,
}

impl ConfigFile {
    pub fn parse(path: impl AsRef<Path>) -> Result<ConfigFile> {
        let config_file = std::fs::File::open(path)?;

        Self::parse_from_reader(config_file)
    }

    pub fn parse_from_reader<R: std::io::Read>(reader: R) -> Result<ConfigFile> {
        let config = serde_yaml::from_reader(reader)?;

        Ok(config)
    }

    pub fn populate_env_variables(&mut self) -> Result<()> {
        self.route
            .pipeline
            .iter_mut()
            .for_each(|item| item.replace().unwrap());

        self.route
            .steps
            .iter_mut()
            .for_each(|item| item.replace().unwrap());

        Ok(())
    }
}
