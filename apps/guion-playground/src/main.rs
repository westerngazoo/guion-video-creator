#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use guion_playground::FrameState;
use guion_playground::{
    export_mp4, preview_png, reset_sim, set_params, sim_info, simulate_to, state_at,
    PlaygroundState, SimInfo,
};
use std::path::PathBuf;

fn main() {
    let playground = PlaygroundState::new().expect("initial pendulum simulation");
    tauri::Builder::default()
        .manage(playground)
        .invoke_handler(tauri::generate_handler![
            cmd_sim_info,
            cmd_reset_sim,
            cmd_set_params,
            cmd_simulate_to,
            cmd_state_at,
            cmd_preview,
            cmd_export_mp4,
        ])
        .run(tauri::generate_context!())
        .expect("error running guion-playground");
}

#[tauri::command]
fn cmd_sim_info(state: tauri::State<PlaygroundState>) -> Result<SimInfo, String> {
    let sim = state.0.lock().map_err(|e| e.to_string())?;
    Ok(sim_info(&sim))
}

#[tauri::command]
fn cmd_reset_sim(state: tauri::State<PlaygroundState>) -> Result<SimInfo, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    let current = guard.clone();
    *guard = reset_sim(&current)?;
    Ok(sim_info(&guard))
}

#[tauri::command]
fn cmd_set_params(
    length: f64,
    gravity: f64,
    angle_deg: f64,
    state: tauri::State<PlaygroundState>,
) -> Result<SimInfo, String> {
    let mut sim = state.0.lock().map_err(|e| e.to_string())?;
    set_params(&mut sim, length, gravity, angle_deg)?;
    Ok(sim_info(&sim))
}

#[tauri::command]
fn cmd_simulate_to(duration: f64, state: tauri::State<PlaygroundState>) -> Result<SimInfo, String> {
    let mut sim = state.0.lock().map_err(|e| e.to_string())?;
    simulate_to(&mut sim, duration)?;
    Ok(sim_info(&sim))
}

#[tauri::command]
fn cmd_state_at(t: f64, state: tauri::State<PlaygroundState>) -> Result<FrameState, String> {
    let sim = state.0.lock().map_err(|e| e.to_string())?;
    Ok(state_at(&sim, t))
}

#[tauri::command]
fn cmd_preview(t: f64, state: tauri::State<PlaygroundState>) -> Result<String, String> {
    let sim = state.0.lock().map_err(|e| e.to_string())?;
    preview_png(&sim, t)
}

#[tauri::command]
fn cmd_export_mp4(
    path: Option<String>,
    state: tauri::State<PlaygroundState>,
) -> Result<String, String> {
    let sim = state.0.lock().map_err(|e| e.to_string())?;
    let out = path.map(PathBuf::from);
    export_mp4(&sim, out)
}
