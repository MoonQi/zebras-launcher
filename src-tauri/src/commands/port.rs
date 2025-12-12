use crate::models::{PortChange, ProjectInfo};
use crate::services::{PortManager, WorkspaceList, WorkspaceService};
use crate::utils::port_checker::is_port_available;
use std::collections::HashSet;

#[tauri::command]
pub async fn check_port_available(port: u16) -> Result<bool, String> {
    Ok(is_port_available(port))
}

#[tauri::command]
pub async fn resolve_port_conflicts(
    current_workspace_id: String,
    mut projects: Vec<ProjectInfo>,
    port_range_start: u16,
    port_range_end: u16,
) -> Result<(Vec<ProjectInfo>, Vec<PortChange>), String> {
    let mut manager = PortManager::new(port_range_start, port_range_end);

    // 收集其他工作区的端口使用情况（排除当前工作区）
    let global_used_ports = collect_global_used_ports(&current_workspace_id)?;

    println!(
        "[PortManager] 检测到其他工作区已使用端口: {:?}",
        global_used_ports
    );

    let changes = manager.resolve_conflicts(&mut projects, &global_used_ports)?;

    // 应用端口变更到配置文件
    if !changes.is_empty() {
        PortManager::apply_port_changes(&changes, &projects)?;
    }

    Ok((projects, changes))
}

/// 收集其他工作区的端口使用情况（排除当前工作区）
fn collect_global_used_ports(current_workspace_id: &str) -> Result<HashSet<u16>, String> {
    let mut used_ports = HashSet::new();

    // 加载所有工作区列表
    let workspace_list = WorkspaceList::load().unwrap_or_else(|_| WorkspaceList {
        workspaces: Vec::new(),
    });

    println!(
        "[PortManager] 找到 {} 个工作区",
        workspace_list.workspaces.len()
    );

    // 遍历每个工作区
    for workspace_ref in workspace_list.workspaces.iter() {
        // 跳过当前工作区
        if workspace_ref.id == current_workspace_id {
            println!("[PortManager] 跳过当前工作区 '{}'", workspace_ref.name);
            continue;
        }

        // 加载工作区配置
        match WorkspaceService::load_workspace(&workspace_ref.config_path) {
            Ok(workspace) => {
                println!(
                    "[PortManager] 加载其他工作区 '{}': {} 个项目",
                    workspace.name,
                    workspace.projects.len()
                );

                // 收集该工作区所有项目的端口
                for project in workspace.projects.iter() {
                    if project.is_valid {
                        used_ports.insert(project.port);
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "[PortManager] 加载工作区 '{}' 失败: {}",
                    workspace_ref.name, e
                );
                // 继续处理其他工作区
                continue;
            }
        }
    }

    Ok(used_ports)
}
