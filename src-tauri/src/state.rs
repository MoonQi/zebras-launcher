use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::models::{Workspace, ProcessInfo};
use crate::services::ProcessManager;

pub struct AppState {
    pub current_workspace: Arc<Mutex<Option<Workspace>>>,
    pub workspaces: Arc<Mutex<HashMap<String, Workspace>>>,
    pub running_processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    pub process_manager: ProcessManager,
}

impl AppState {
    pub fn new(window: tauri::Window) -> Self {
        Self {
            current_workspace: Arc::new(Mutex::new(None)),
            workspaces: Arc::new(Mutex::new(HashMap::new())),
            running_processes: Arc::new(Mutex::new(HashMap::new())),
            process_manager: ProcessManager::new(window),
        }
    }
}
