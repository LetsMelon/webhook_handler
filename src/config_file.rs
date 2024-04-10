use std::collections::HashMap;
use std::convert::Infallible;
use std::path::Path;
use std::str::FromStr;

use cron::Schedule;

#[derive(Debug, Clone)]
pub enum ConfigVersion {
    V1_0Beta,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub expose: u16,
    pub uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub uses: String,
    pub name: Option<String>,
    pub with: HashMap<String, String>, // TODO maybe make the value type generic over sth
    pub arguments: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub period: Schedule, // TODO the struct `Schedule` is really large, maybe box or rc/arc it?
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub pipeline: Vec<Step>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub version: ConfigVersion,
    pub config: Config,
    pub health_check: Option<HealthCheck>,
    pub route: Route,
}

pub fn parse_config_file(_path: impl AsRef<Path>) -> Result<ConfigFile, Infallible> {
    // every 5 minutes
    let expression = "0 5 * * * * *";

    Ok(ConfigFile {
        version: ConfigVersion::V1_0Beta,
        config: Config {
            expose: 3000,
            uri: Some("https://webhook.melcher.io".to_string()),
        },
        health_check: Some(HealthCheck {
            period: Schedule::from_str(expression).unwrap(),
            steps: vec![Step {
                uses: "docker/ping".to_string(),
                name: None,
                with: HashMap::new(),
                env: HashMap::new(),
            }],
        }),
        route: Route {
            path: "/github".to_string(),
            pipeline: vec![Step {
                uses: "http_pipeline_wasm".to_string(),
                name: None,
                with: {
                    let mut map = HashMap::new();

                    map.insert(
                        "wasm".to_string(),
                        "./github_accept_webhook.wasm".to_string(),
                    );

                    map
                },
                env: {
                    let mut map = HashMap::new();

                    map.insert("github_key".to_string(), "${{ GITHUB_KEY }}".to_string());

                    map
                },
            }],
            steps: vec![
                Step {
                    uses: "docker/stop_container".to_string(),
                    name: Some("Stop the container".to_string()),
                    with: {
                        let mut map = HashMap::new();

                        map.insert("container_name".to_string(), "my_website".to_string());

                        map
                    },
                    env: HashMap::new(),
                },
                Step {
                    uses: "docker/build_image".to_string(),
                    name: Some("Build the new image".to_string()),
                    with: {
                        let mut map = HashMap::new();

                        map.insert("image_name".to_string(), "my_website_image".to_string());
                        map.insert("dockerfile".to_string(), "./Dockerfile.auto".to_string());

                        map
                    },
                    env: HashMap::new(),
                },
                Step {
                    uses: "docker/start_image".to_string(),
                    name: Some("Start new image as container".to_string()),
                    with: {
                        let mut map = HashMap::new();

                        map.insert("container_name".to_string(), "my_website".to_string());
                        map.insert("image_name".to_string(), "my_website_image".to_string());
                        map.insert(
                            "networks".to_string(),
                            "personal_website_internal_network".to_string(),
                        );
                        map.insert("ports".to_string(), "8080:80".to_string());
                        map.insert("auto_remove".to_string(), true.to_string());

                        map
                    },
                    env: HashMap::new(),
                },
            ],
        },
    })
}
