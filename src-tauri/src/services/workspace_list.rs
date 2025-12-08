use crate::models::Workspace;
use crate::services::WorkspaceService;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceList {
    pub workspaces: Vec<WorkspaceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRef {
    pub id: String,
    pub name: String,
    pub config_path: PathBuf,
    pub last_opened: Option<String>,
}

impl WorkspaceList {
    /// 获取工作区列表文件路径
    fn get_list_path() -> Result<PathBuf, String> {
        let home = dirs_next::home_dir()
            .ok_or("无法获取用户主目录".to_string())?;

        let config_dir = home.join(".zebras-launcher");

        // 确保目录存在
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;

        Ok(config_dir.join("workspaces.json"))
    }

    /// 加载工作区列表（包含自动迁移旧配置）
    pub fn load() -> Result<Self, String> {
        let list_path = Self::get_list_path()?;

        if !list_path.exists() {
            return Ok(Self {
                workspaces: Vec::new(),
            });
        }

        let content = fs::read_to_string(&list_path)
            .map_err(|e| format!("读取工作区列表失败: {}", e))?;

        let mut list: Self = serde_json::from_str(&content)
            .map_err(|e| format!("解析工作区列表失败: {}", e))?;

        // 自动迁移旧配置
        list.migrate_old_configs()?;

        Ok(list)
    }

    /// 迁移旧配置文件到新位置
    fn migrate_old_configs(&mut self) -> Result<(), String> {
        let mut needs_save = false;

        for workspace_ref in self.workspaces.iter_mut() {
            // 检查配置文件是否存在
            let new_path = WorkspaceService::get_config_path(&workspace_ref.id)?;

            // 如果新路径已存在，只需更新列表中的路径
            if new_path.exists() {
                if workspace_ref.config_path != new_path {
                    workspace_ref.config_path = new_path;
                    needs_save = true;
                }
                continue;
            }

            // 如果新路径不存在，检查旧路径是否存在
            let old_path = &workspace_ref.config_path;
            if old_path.exists() {
                // 尝试从旧路径加载工作区
                match WorkspaceService::load_workspace(old_path) {
                    Ok(workspace) => {
                        // 保存到新位置
                        if let Err(e) = WorkspaceService::save_workspace(&workspace) {
                            eprintln!("迁移工作区 {} 失败: {}", workspace_ref.name, e);
                            continue;
                        }

                        // 更新列表中的路径
                        workspace_ref.config_path = new_path;
                        needs_save = true;

                        println!("已迁移工作区: {} -> {}", workspace_ref.name, workspace_ref.config_path.display());
                    }
                    Err(e) => {
                        eprintln!("无法加载旧工作区 {}: {}", workspace_ref.name, e);
                    }
                }
            }
        }

        // 如果有更新，保存列表
        if needs_save {
            self.save()?;
        }

        Ok(())
    }

    /// 保存工作区列表
    pub fn save(&self) -> Result<(), String> {
        let list_path = Self::get_list_path()?;

        let json = serde_json::to_string_pretty(&self)
            .map_err(|e| format!("序列化工作区列表失败: {}", e))?;

        fs::write(&list_path, json)
            .map_err(|e| format!("写入工作区列表失败: {}", e))?;

        Ok(())
    }

    /// 添加工作区
    pub fn add_workspace(&mut self, workspace: &Workspace) -> Result<(), String> {
        // 使用新的集中式存储路径
        let config_path = WorkspaceService::get_config_path(&workspace.id)?;

        // 检查是否已存在
        if self.workspaces.iter().any(|w| w.id == workspace.id) {
            return Ok(()); // 已存在，不重复添加
        }

        self.workspaces.push(WorkspaceRef {
            id: workspace.id.clone(),
            name: workspace.name.clone(),
            config_path,
            last_opened: Some(chrono::Utc::now().to_rfc3339()),
        });

        self.save()?;
        Ok(())
    }

    /// 移除工作区
    pub fn remove_workspace(&mut self, workspace_id: &str) -> Result<(), String> {
        self.workspaces.retain(|w| w.id != workspace_id);
        self.save()?;
        Ok(())
    }

    /// 更新最后打开时间
    pub fn update_last_opened(&mut self, workspace_id: &str) -> Result<(), String> {
        if let Some(workspace_ref) = self.workspaces.iter_mut().find(|w| w.id == workspace_id) {
            workspace_ref.last_opened = Some(chrono::Utc::now().to_rfc3339());
            self.save()?;
        }
        Ok(())
    }
}
