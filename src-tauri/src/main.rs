#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod models;
mod services;
mod state;
mod utils;

use state::AppState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            app.manage(AppState::new(window.clone()));

            // 监听窗口关闭事件，自动清理进程
            let app_handle = app.handle();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let state = app_handle.state::<AppState>();
                    // 阻塞式停止所有进程
                    tauri::async_runtime::block_on(async {
                        let _ = state.process_manager.stop_all().await;
                        let _ = state.terminal_manager.stop_all().await;
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Workspace commands
            commands::create_workspace,
            commands::load_workspace,
            commands::scan_workspace_projects,
            commands::save_workspace,
            commands::delete_workspace,
            commands::add_workspace_folder,
            commands::remove_workspace_folder,
            commands::get_workspace_list,
            commands::update_project_enabled,
            // Port commands
            commands::check_port_available,
            commands::resolve_port_conflicts,
            // Project commands
            commands::get_project_details,
            commands::rescan_project,
            commands::is_zebras_project,
            // Process commands
            commands::start_project,
            commands::stop_project,
            commands::get_running_processes,
            commands::stop_all_projects,
            commands::start_all_projects,
            commands::run_project_task,
            // Terminal commands
            commands::create_terminal_session,
            commands::get_terminal_sessions,
            commands::run_terminal_command,
            commands::kill_terminal_session,
            commands::close_terminal_session,
            // Git commands
            commands::is_git_repo,
            commands::get_git_status,
            commands::git_fetch,
            commands::git_pull,
            // Debug commands
            commands::update_debug_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
