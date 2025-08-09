use tracing::info;

/// Set up graceful shutdown signal handling
pub async fn setup_shutdown_signal() {
    use tokio::signal;

    #[cfg(unix)]
    {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT"),
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.unwrap();
        info!("Received Ctrl+C");
    }
}
