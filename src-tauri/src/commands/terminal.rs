use crate::models::TerminalSession;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_terminal_session(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<TerminalSession, String> {
    state.terminal_manager.create_session(project_id).await
}

#[tauri::command]
pub async fn get_terminal_sessions(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TerminalSession>, String> {
    Ok(state.terminal_manager.get_sessions(project_id).await)
}

#[tauri::command]
pub async fn run_terminal_command(
    session_id: String,
    project_path: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .terminal_manager
        .run_command(session_id, project_path, command)
        .await
}

#[tauri::command]
pub async fn kill_terminal_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.terminal_manager.kill_session(&session_id).await
}

#[tauri::command]
pub async fn close_terminal_session(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.terminal_manager.close_session(&session_id).await
}
