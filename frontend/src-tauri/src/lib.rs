use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Data (graph, sims, settings) lives in the OS app-data dir; the
            // built frontend is served BY the backend, so UI + API share one
            // loopback origin — no separate server, no CORS.
            if let Ok(data) = app.path().app_data_dir() {
                std::fs::create_dir_all(&data).ok();
                std::env::set_var("GRAPH_DB_PATH", data.join("graph.db"));
                std::env::set_var("OASIS_SIMULATION_DATA_DIR", data.join("simulations"));
                std::env::set_var("SETTINGS_PATH", data.join("settings.json"));
            }
            std::env::set_var("STATIC_DIR", concat!(env!("CARGO_MANIFEST_DIR"), "/../dist"));

            // Bind the loopback port up front so the window knows its URL. A bound
            // std listener already accepts into the backlog, so the window may load
            // before axum starts serving — the request just waits, no refused error.
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            listener.set_nonblocking(true).ok();
            let port = listener.local_addr().unwrap().port();
            let url = format!("http://127.0.0.1:{port}");

            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(listener).expect("tokio listener");
                popinion::serve(listener, popinion::Config::from_env()).await;
            });

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().unwrap()))
                .title("Popinion — Public Opinion Analysis")
                .inner_size(1400.0, 950.0)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
