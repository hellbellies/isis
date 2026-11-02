use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::models::settings::Settings;
use crate::services;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppResult<Settings> {
    let dir = app.path().app_config_dir()?;
    services::settings::load(&dir)
}

#[tauri::command]
pub fn update_settings(app: AppHandle, settings: Settings) -> AppResult<Settings> {
    let dir = app.path().app_config_dir()?;
    services::settings::save(&dir, &settings)
}
