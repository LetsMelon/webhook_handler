use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use cron::Schedule;
use derivative::Derivative;
use glue::error::CustomError;
use glue::exports::fct_setup;
use tokio::sync::Mutex;
use tracing::span;
use uuid::Uuid;
use wasmtime::{Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{
    HostOutputStream, StdoutStream, StreamResult, Subscribe, WasiCtxBuilder, WasiP1Ctx,
};

use crate::raw::{Config, ConfigFile, ConfigVersion, Route, Step};

#[derive(Debug)]
enum Variable<'a> {
    Env(&'a str),
}

#[derive(Debug, Clone)]
struct LogStream {
    name: String,
    span: tracing::Span,
}

impl LogStream {
    fn new(name: String) -> Self {
        // TODO set target to sth like 'wasm_module', so that I don't have to enable tracing for this crate in the subscriber
        let span = span!(tracing::Level::INFO, "wasm_module", name);

        // let metadata = Metadata::new(
        //     &name,
        //     "wasm_module",
        //     log_level,
        //     None,
        //     None,
        //     None,
        //     fields,
        //     tracing::metadata::Kind::SPAN,
        // );

        // tracing::Span::child_of(
        //     current_span,
        //     &metadata,
        //     &tracing::valueset! { metadata.fields(), target, name, file, line, module_path, ?params },
        // ),

        LogStream {
            name: name.clone(),
            span,
        }
    }
}

impl StdoutStream for LogStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        true
    }
}

impl HostOutputStream for LogStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        let _enter = self.span.enter();
        let msg = String::from_utf8_lossy(bytes.as_ref());

        let msg = msg.trim();

        // TODO maybe use something like json or postcard to send the message, and then have the level separate from the actual message
        let is_info = msg.strip_prefix("INFO").map(|item| item.trim());
        let is_debug = msg.strip_prefix("DEBUG").map(|item| item.trim());
        let is_error = msg.strip_prefix("ERROR").map(|item| item.trim());
        let is_trace = msg.strip_prefix("TRACE").map(|item| item.trim());
        let is_warn = msg.strip_prefix("WARN").map(|item| item.trim());

        // TODO is there a better way to specify the level of the event?
        match (is_info, is_debug, is_error, is_trace, is_warn) {
            (Some(msg), _, _, _, _) => tracing::info!("{}: {}", self.name, msg),
            (_, Some(msg), _, _, _) => tracing::debug!("{}: {}", self.name, msg),
            (_, _, Some(msg), _, _) => tracing::error!("{}: {}", self.name, msg),
            (_, _, _, Some(msg), _) => tracing::trace!("{}: {}", self.name, msg),
            (_, _, _, _, Some(msg)) => tracing::warn!("{}: {}", self.name, msg),
            _ => tracing::info!("{}: {}", self.name, msg),
        }

        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(1024)
    }
}

#[async_trait::async_trait]
impl Subscribe for LogStream {
    async fn ready(&mut self) {}
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

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct StepInternal {
    pub uses: String,
    pub name: Option<String>,
    pub with: HashMap<String, String>,
    pub arguments: HashMap<String, String>,

    pub id: Uuid,
    #[derivative(Debug = "ignore")]
    pub instance: Option<Arc<Instance>>,
    #[derivative(Debug = "ignore")]
    pub store: Option<Arc<Mutex<Store<WasiP1Ctx>>>>,
}

impl StepInternal {
    async fn from_step(value: Step) -> Result<StepInternal> {
        let mut step = StepInternal {
            uses: value.uses,
            name: value.name,
            with: value.with,
            arguments: value.arguments,
            id: Uuid::new_v4(),
            instance: None,
            store: None,
        };

        if let Some(wasm_module) = step.with.get("wasm") {
            let engine = Engine::new(
                wasmtime::Config::default()
                    .async_support(true)
                    .dynamic_memory_guard_size(1 << 20),
            )?;

            let mut linker = Linker::new(&engine);
            wasmtime_wasi::preview1::add_to_linker_async(&mut linker, |s| s)?;

            let log_stdout = LogStream::new("stdout".to_string());
            let log_stderr = LogStream::new("stderr".to_string());

            let wasi = WasiCtxBuilder::new()
                .stdout(log_stdout)
                .stderr(log_stderr)
                .build_p1();
            let mut store = Store::new(&engine, wasi);

            let module_validator = Module::from_file(&engine, wasm_module)?;
            linker
                .module_async(&mut store, &step.id.to_string(), &module_validator)
                .await?;

            let instance = linker
                .instantiate_async(&mut store, &module_validator)
                .await?;

            let instance = Arc::new(instance);
            let store = Arc::new(Mutex::new(store));

            let fct_setup = fct_setup(instance.clone(), store.clone()).await?;
            if fct_setup().await? != 0 {
                let error = CustomError::from_wasm(instance.clone(), store.clone())
                    .await?
                    .context("Could not get the error from wasm")?;
                dbg!(error);

                panic!("Can't init the wasm module");
            }

            step.instance = Some(instance);
            step.store = Some(store);
        }

        Ok(step)
    }
}

impl ReplaceVariables for StepInternal {
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

#[derive(Debug, Clone)]
pub struct HealthCheckInternal {
    pub period: Schedule, // TODO the struct `Schedule` is really large, maybe box or rc/arc it?
    pub steps: Vec<StepInternal>,
}

#[derive(Debug, Clone)]
pub struct RouteInternal {
    pub path: String,
    pub pipeline: Vec<StepInternal>,
    pub steps: Vec<StepInternal>,
}

impl RouteInternal {
    async fn from_route(value: Route) -> Result<RouteInternal> {
        let mut pipeline_internal = Vec::with_capacity(value.pipeline.len());
        for pipeline in value.pipeline {
            pipeline_internal.push(StepInternal::from_step(pipeline).await?);
        }

        let mut steps = Vec::with_capacity(value.steps.len());
        for step in value.steps {
            steps.push(StepInternal::from_step(step).await?);
        }

        Ok(RouteInternal {
            path: value.path,
            pipeline: pipeline_internal,
            steps,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConfigFileInternal {
    pub version: ConfigVersion,
    pub config: Config,
    pub health_check: Option<HealthCheckInternal>,
    pub route: RouteInternal,
}

impl ConfigFileInternal {
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

    pub async fn from_config(value: ConfigFile) -> Result<ConfigFileInternal> {
        let health_check = if let Some(health_check) = value.health_check {
            let mut steps_internal = Vec::with_capacity(health_check.steps.len());

            for step in health_check.steps {
                steps_internal.push(StepInternal::from_step(step).await?);
            }

            Some(HealthCheckInternal {
                period: health_check.period,
                steps: steps_internal,
            })
        } else {
            None
        };

        Ok(ConfigFileInternal {
            version: value.version,
            config: value.config,
            health_check,
            route: RouteInternal::from_route(value.route).await?,
        })
    }
}
