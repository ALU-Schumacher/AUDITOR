use auditor::configuration::get_configuration;
use auditor::startup::run;
use auditor::telemetry::{get_subscriber, init_subscriber};
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Set up logging
    let subscriber = get_subscriber("AUDITOR".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    // Read in configuration
    let configuration = get_configuration().expect("Failed to read configuration.");

    // Create a connection pool for the PostgreSQL database
    let connection_pool = PgPoolOptions::new()
        .connect_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(configuration.database.connection_string().expose_secret())
        .expect("Failed to create Postgres connection pool.");

    // Create a TcpListener for a given address and port
    let address = format!(
        "{}:{}",
        configuration.application.addr, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;

    // Start server
    run(listener, connection_pool)?.await?;

    Ok(())
}
