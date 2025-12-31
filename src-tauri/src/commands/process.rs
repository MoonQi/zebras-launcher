use crate::models::{ProcessInfo, Workspace};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn start_project(
    project_id: String,
    project_name: String,
    project_path: String,
    state: State<'_, AppState>,
) -> Result<ProcessInfo, String> {
    let process_info = state
        .process_manager
        .start_project(project_id, project_name, project_path)
        .await?;

    // 保存到全局状态
    state
        .running_processes
        .lock()
        .await
        .insert(process_info.process_id.clone(), process_info.clone());

    Ok(process_info)
}

#[tauri::command]
pub async fn stop_project(process_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.process_manager.stop_project(&process_id).await?;

    // 从全局状态移除
    state.running_processes.lock().await.remove(&process_id);

    Ok(())
}

#[tauri::command]
pub async fn get_running_processes(state: State<'_, AppState>) -> Result<Vec<ProcessInfo>, String> {
    let processes = state.running_processes.lock().await;
    Ok(processes.values().cloned().collect())
}

#[tauri::command]
pub async fn stop_all_projects(state: State<'_, AppState>) -> Result<(), String> {
    state.process_manager.stop_all().await?;

    // 清空全局状态
    state.running_processes.lock().await.clear();

    Ok(())
}

#[tauri::command]
pub async fn start_all_projects(
    workspace: Workspace,
    state: State<'_, AppState>,
) -> Result<Vec<ProcessInfo>, String> {
    let mut started_processes = Vec::new();

    for project in workspace.projects.iter() {
        // 只启动有效且已启用的项目
        if !project.is_valid {
            continue;
        }

        // 检查项目是否启用（enabled 为 None 或 Some(true) 时启动）
        if let Some(false) = project.enabled {
            continue; // 明确禁用的项目跳过
        }

        match state
            .process_manager
            .start_project(
                project.id.clone(),
                project.name.clone(),
                project.path.to_string_lossy().to_string(),
            )
            .await
        {
            Ok(process_info) => {
                // 保存到全局状态
                state
                    .running_processes
                    .lock()
                    .await
                    .insert(process_info.process_id.clone(), process_info.clone());
                started_processes.push(process_info);
            }
            Err(e) => {
                eprintln!("启动项目 {} 失败: {}", project.name, e);
                // 继续启动其他项目
                continue;
            }
        }
    }

    Ok(started_processes)
}

#[tauri::command]
pub async fn run_project_task(
    project_id: String,
    project_name: String,
    project_path: String,
    task: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .process_manager
        .run_task(project_id, project_name, project_path, task)
        .await
}
