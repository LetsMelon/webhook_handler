use std::sync::Arc;

use anyhow::Result;

mod server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    let config_raw = config_parser::raw::ConfigFile::parse("./webhook_handler_demo_config.yml")?;
    let mut config = config_parser::internal::ConfigFileInternal::from_config(config_raw).await?;
    config.populate_env_variables()?;
    let config = Arc::new(config);

    let server_handle = tokio::spawn({
        let config = config.clone();

        println!("Server is starting");

        async { crate::server::start(config).await }
    });

    server_handle.await??;

    Ok(())
}
