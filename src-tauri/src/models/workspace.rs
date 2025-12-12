use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: PathBuf,   // 工作区配置文件的存储路径
    pub folders: Vec<String>, // 包含的多个代码文件夹路径
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub projects: Vec<super::ProjectInfo>,
    pub settings: WorkspaceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    pub auto_start_all: bool,
    pub port_strategy: PortStrategy,
    pub port_range_start: u16,
    pub port_range_end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortStrategy {
    Sequential,
    Fixed,
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            auto_start_all: false,
            port_strategy: PortStrategy::Sequential,
            port_range_start: 8000,
            port_range_end: 9000,
        }
    }
}

impl Workspace {
    pub fn new(name: String, root_path: PathBuf) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            root_path,
            folders: Vec::new(), // 初始为空，后续添加文件夹
            created_at: Utc::now(),
            last_modified: Utc::now(),
            projects: Vec::new(),
            settings: WorkspaceSettings::default(),
        }
    }

    pub fn add_folder(&mut self, folder_path: String) {
        if !self.folders.contains(&folder_path) {
            self.folders.push(folder_path);
            self.last_modified = Utc::now();
        }
    }

    pub fn remove_folder(&mut self, folder_path: &str) {
        self.folders.retain(|f| f != folder_path);
        self.last_modified = Utc::now();
    }
}
