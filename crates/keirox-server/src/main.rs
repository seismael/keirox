//! # Keirox Server
//!
//! Production distributed daemon and CLI entry point for the Keirox runtime.

use clap::Parser;
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    name = "keirox-server",
    version,
    about = "Keirox Polymorphic Event Fabric Runtime Daemon"
)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config/keirox.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!(
        config = %args.config,
        "Starting Keirox Polymorphic Event Fabric Server"
    );

    Ok(())
}
