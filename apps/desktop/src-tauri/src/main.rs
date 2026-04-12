mod commands;
mod db;
mod sidecar;
mod state;

use state::AppState;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            use tauri::Manager;
            let app_data = app.path().app_data_dir().expect("failed to get app data dir");
            let data_dir = app_data.join("data");
            std::fs::create_dir_all(&data_dir)?;

            let db_path = data_dir.join("fanthom.db");
            let conn = db::open(&db_path).expect("failed to open database");

            app.manage(AppState::new(conn, data_dir));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::pipeline::process_url,
            commands::tab::transpose,
            commands::tab::toggle_optimization,
            commands::tab::regenerate_tab,
            commands::tab::export_tab,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
