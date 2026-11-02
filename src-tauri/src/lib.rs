use tauri::Manager;

mod commands;
mod db;
mod error;
mod models;
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let conn = db::open(&app.path().app_data_dir()?)?;
            app.manage(db::Db::new(conn));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::jira::get_jira_settings,
            commands::jira::update_jira_settings,
            commands::jira::clear_jira_token,
            commands::jira::test_jira_connection,
            commands::jira::list_jira_issues,
            commands::jira::sync_jira_issues,
            commands::jira::open_jira_issue,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
