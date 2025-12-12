use crate::models::Workspace;
use std::fs;
use std::path::{Path, PathBuf};

pub struct WorkspaceService;

impl WorkspaceService {
    /// 获取工作区配置目录
    fn get_workspaces_dir() -> Result<PathBuf, String> {
        let home = dirs_next::home_dir().ok_or("无法获取用户主目录".to_string())?;

        let workspaces_dir = home.join(".zebras-launcher").join("workspaces");

        // 确保目录存在
        fs::create_dir_all(&workspaces_dir).map_err(|e| format!("创建工作区目录失败: {}", e))?;

        Ok(workspaces_dir)
    }

    /// 获取工作区配置文件路径
    fn get_workspace_config_path(workspace_id: &str) -> Result<PathBuf, String> {
        let workspaces_dir = Self::get_workspaces_dir()?;
        Ok(workspaces_dir.join(format!("{}.json", workspace_id)))
    }

    /// 加载工作区配置文件
    pub fn load_workspace(workspace_path: &Path) -> Result<Workspace, String> {
        if !workspace_path.exists() {
            return Err("工作区文件不存在".to_string());
        }

        let content =
            fs::read_to_string(workspace_path).map_err(|e| format!("读取工作区文件失败: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("解析工作区配置失败: {}", e))
    }

    /// 保存工作区配置到用户目录
    pub fn save_workspace(workspace: &Workspace) -> Result<(), String> {
        let workspace_file = Self::get_workspace_config_path(&workspace.id)?;

        let json = serde_json::to_string_pretty(workspace)
            .map_err(|e| format!("序列化工作区配置失败: {}", e))?;

        fs::write(&workspace_file, json).map_err(|e| format!("写入工作区文件失败: {}", e))?;

        Ok(())
    }

    /// 删除工作区配置文件
    pub fn delete_workspace(workspace_id: &str) -> Result<(), String> {
        let workspace_path = Self::get_workspace_config_path(workspace_id)?;

        if workspace_path.exists() {
            fs::remove_file(workspace_path).map_err(|e| format!("删除工作区文件失败: {}", e))?;
        }
        Ok(())
    }

    /// 检查工作区文件是否存在
    pub fn workspace_exists(workspace_id: &str) -> bool {
        if let Ok(path) = Self::get_workspace_config_path(workspace_id) {
            path.exists()
        } else {
            false
        }
    }

    /// 获取工作区配置文件路径（用于WorkspaceList）
    pub fn get_config_path(workspace_id: &str) -> Result<PathBuf, String> {
        Self::get_workspace_config_path(workspace_id)
    }
}
