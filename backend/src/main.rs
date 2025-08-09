use anyhow::Result;
use blackledger::{app, config};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    app::init_tracing();

    // Load configuration
    let config = config::Config::from_env()?;
    let port = config.port;

    // Build the application
    let application = app::build_app(config).await?;

    // Start the server
    app::start_server(application, port).await?;

    Ok(())
}
