use std::sync::Arc;

use anyhow::Result;
use tracing::*;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

mod server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or(
                "webhook_handler=debug,hyper=info,wasmtime=info,config_parser=trace".into(),
            ),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let config_raw = config_parser::raw::ConfigFile::parse("./webhook_handler_demo_config.yml")?;
    let mut config = config_parser::internal::ConfigFileInternal::from_config(config_raw).await?;
    config.populate_env_variables()?;
    let config = Arc::new(config);

    let server_handle = tokio::spawn({
        let config = config.clone();

        info!("Server is starting");

        async { crate::server::start(config).await }
    });

    server_handle.await??;

    Ok(())
}
