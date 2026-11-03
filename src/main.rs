use std::{error::Error, net::SocketAddr, time::Duration};

use axum_server::Handle;
use store_app::{AppState, app, config::Config, db, tls};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_tracing();

    let config = Config::from_env()?;
    tls::install_crypto_provider();

    let pool = db::connect(&config).await?;
    let app = app(AppState { pool });

    let tls_config = tls::config(&config).await?;
    let addr = SocketAddr::new(config.host, config.port);
    let handle = Handle::new();

    tokio::spawn(shutdown(handle.clone()));

    tracing::info!(
        address = %addr,
        protocol = "https",
        "Store App is listening"
    );

    axum_server::bind_rustls(addr, tls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await?;

    tracing::info!("Store App stopped");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("store_app=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn shutdown(handle: Handle<SocketAddr>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("the interrupt handler can be installed");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("the terminate handler can be installed")
            .recv()
            .await;
    };

    #[cfg(unix)]
    tokio::select! {
        () = ctrl_c => {},
        _ = terminate => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await;

    tracing::info!("shutdown signal received");
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
