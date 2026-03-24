use tokio_util::sync::CancellationToken;

pub fn register_ctrlc_handler(
    runtime: &mut tokio::runtime::Runtime,
    cancellation_token: CancellationToken,
) {
    let cancel_token = cancellation_token.clone();
    runtime.handle().spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c signal");
        info!("Ctrl-C received, shutting down...");
        cancel_token.cancel();
    });
}
