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

    // Bind loopback by default so the unauthenticated API isn't exposed to the
    // LAN; set HOST=0.0.0.0 to deliberately expose it (behind your own auth/proxy).
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let listener = tokio::net::TcpListener::bind((host.as_str(), port))
        .await
        .expect("bind port");
    tracing::info!("Popinion Backend listening on http://{host}:{port}");
    axum::serve(listener, app).await.expect("server");
}
