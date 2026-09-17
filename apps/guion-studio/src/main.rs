#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use guion_studio::{
    encode_screenplay, list_templates, load_screenplay, narrate_screenplay, preview_frame,
    render_screenplay, templates_dir,
};
use std::path::PathBuf;

#[tauri::command]
fn cmd_load_screenplay(path: String) -> Result<guion_studio::ScreenplayInfo, String> {
    load_screenplay(PathBuf::from(path).as_path())
}

#[tauri::command]
fn cmd_list_templates() -> Result<Vec<guion_studio::TemplateEntry>, String> {
    list_templates(&templates_dir())
}

#[tauri::command]
fn cmd_preview(path: String, t: f64) -> Result<String, String> {
    preview_frame(PathBuf::from(path).as_path(), t)
}

#[tauri::command]
fn cmd_render(path: String) -> Result<String, String> {
    render_screenplay(PathBuf::from(path).as_path(), None, None)
}

#[tauri::command]
fn cmd_narrate(path: String, engine: Option<String>) -> Result<String, String> {
    narrate_screenplay(
        PathBuf::from(path).as_path(),
        engine.as_deref().unwrap_or("scaffold"),
    )
}

#[tauri::command]
fn cmd_encode(path: String) -> Result<String, String> {
    encode_screenplay(PathBuf::from(path).as_path(), None)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd_load_screenplay,
            cmd_list_templates,
            cmd_preview,
            cmd_render,
            cmd_narrate,
            cmd_encode,
        ])
        .run(tauri::generate_context!())
        .expect("error running guion-studio");
}
