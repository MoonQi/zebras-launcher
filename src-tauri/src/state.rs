use crate::models::ProcessInfo;
use crate::services::{ProcessManager, TerminalManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub running_processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    pub process_manager: ProcessManager,
    pub terminal_manager: TerminalManager,
}

impl AppState {
    pub fn new(window: tauri::Window) -> Self {
        Self {
            running_processes: Arc::new(Mutex::new(HashMap::new())),
            process_manager: ProcessManager::new(window.clone()),
            terminal_manager: TerminalManager::new(window),
        }
    }
}
