use popinion::{build_app, Config};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "popinion=debug,tower_http=warn,info".into()),
        )
        .init();

    let cfg = Config::from_env();
    let port = cfg.port;
    tracing::info!("Popinion Backend starting on :{port}");

    let app = build_app(cfg).await;

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .expect("bind port");
    tracing::info!("Popinion Backend listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("server");
}
