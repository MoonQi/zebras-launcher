use crate::models::{ProcessInfo, ProcessStatus};
use crate::utils::kill_process_tree;
use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(not(target_os = "windows"))]
use crate::utils::{resolve_program_in_user_path, USER_PATH};

#[derive(Clone, Serialize)]
pub struct LogMessage {
    pub process_id: String,
    pub session_id: Option<String>,
    pub project_id: String,
    pub project_name: String,
    pub message: String,
    pub stream: String, // "stdout" or "stderr"
}

pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    window: tauri::Window,
}

struct ProcessHandle {
    child: Child,
    project_name: String,
    project_path: String,
}

impl ProcessManager {
    pub fn new(window: tauri::Window) -> Self {
        // 在创建时预热 PATH 缓存
        #[cfg(not(target_os = "windows"))]
        {
            let _ = &*USER_PATH;
        }

        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            window,
        }
    }

    /// 查找 npm 命令路径 (仅 Windows 使用)
    #[cfg(target_os = "windows")]
    fn find_npm_command() -> Result<String, String> {
        Ok("npm.cmd".to_string())
    }

    #[cfg(target_os = "windows")]
    fn find_pnpm_command() -> Result<String, String> {
        Ok("pnpm.cmd".to_string())
    }

    /// 启动项目
    pub async fn start_project(
        &self,
        project_id: String,
        project_name: String,
        project_path: String,
    ) -> Result<ProcessInfo, String> {
        let process_id = uuid::Uuid::new_v4().to_string();

        // 创建命令（跨平台处理）
        #[cfg(target_os = "windows")]
        let mut child = {
            let npm_cmd = Self::find_npm_command()?;
            let mut command = Command::new(&npm_cmd);
            command
                .args(&["run", "start"])
                .current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);

            command
                .spawn()
                .map_err(|e| format!("启动项目失败: {}", e))?
        };

        #[cfg(not(target_os = "windows"))]
        let mut child = {
            // macOS/Linux: 使用缓存的用户 PATH 环境变量
            // 这个 PATH 是从用户的 login shell 获取的，包含了所有全局命令路径
            let program_path =
                resolve_program_in_user_path("npm").unwrap_or_else(|| "npm".to_string());
            let mut command = Command::new(program_path);
            command
                .args(&["run", "start"])
                .current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("PATH", &*USER_PATH); // 使用用户终端的完整 PATH

            command
                .spawn()
                .map_err(|e| format!("启动项目失败: {}", e))?
        };

        let pid = child.id();

        // 捕获 stdout 和 stderr
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // 存储进程句柄
        let handle = ProcessHandle {
            child,
            project_name: project_name.clone(),
            project_path: project_path.clone(),
        };

        self.processes
            .lock()
            .await
            .insert(process_id.clone(), handle);

        // 启动日志流任务
        if let Some(stdout) = stdout {
            let process_id_clone = process_id.clone();
            let project_id_clone = project_id.clone();
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(tokio::process::ChildStdout::from_std(stdout).unwrap());
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
                        session_id: None,
                        project_id: project_id_clone.clone(),
                        project_name: project_name_clone.clone(),
                        message: line,
                        stream: "stdout".to_string(),
                    };
                    let _ = window_clone.emit("process_log", &log_msg);
                }
            });
        }

        if let Some(stderr) = stderr {
            let process_id_clone = process_id.clone();
            let project_id_clone = project_id.clone();
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(tokio::process::ChildStderr::from_std(stderr).unwrap());
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
                        session_id: None,
                        project_id: project_id_clone.clone(),
                        project_name: project_name_clone.clone(),
                        message: line,
                        stream: "stderr".to_string(),
                    };
                    let _ = window_clone.emit("process_log", &log_msg);
                }
            });
        }

        Ok(ProcessInfo {
            process_id: process_id.clone(),
            project_id,
            project_name,
            status: ProcessStatus::Running,
            started_at: Utc::now(),
            pid: Some(pid),
        })
    }

    /// 运行项目的快捷任务（等待完成）
    pub async fn run_task(
        &self,
        project_id: String,
        project_name: String,
        project_path: String,
        task: String,
    ) -> Result<(), String> {
        let (program, args): (&str, Vec<&str>) = match task.as_str() {
            "npm_install" => ("npm", vec!["install"]),
            "pnpm_install" => ("pnpm", vec!["install"]),
            "npm_deploy" => ("npm", vec!["run", "deploy"]),
            _ => return Err("不支持的任务类型".to_string()),
        };

        let process_id = uuid::Uuid::new_v4().to_string();

        #[cfg(target_os = "windows")]
        let mut command = {
            let program_name = match program {
                "npm" => Self::find_npm_command()?,
                "pnpm" => Self::find_pnpm_command()?,
                _ => program.to_string(),
            };
            let mut cmd = TokioCommand::new(program_name);
            cmd.args(&args)
                .current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        };

        #[cfg(not(target_os = "windows"))]
        let mut command = {
            let program_path =
                resolve_program_in_user_path(program).unwrap_or_else(|| program.to_string());
            let mut cmd = TokioCommand::new(program_path);
            cmd.args(&args)
                .current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("PATH", &*USER_PATH);
            cmd
        };

        let mut child = command
            .spawn()
            .map_err(|e| format!("执行命令失败: {}", e))?;

        if let Some(stdout) = child.stdout.take() {
            let process_id_clone = process_id.clone();
            let project_id_clone = project_id.clone();
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
                        session_id: None,
                        project_id: project_id_clone.clone(),
                        project_name: project_name_clone.clone(),
                        message: line,
                        stream: "stdout".to_string(),
                    };
                    let _ = window_clone.emit("process_log", &log_msg);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let process_id_clone = process_id.clone();
            let project_id_clone = project_id.clone();
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
                        session_id: None,
                        project_id: project_id_clone.clone(),
                        project_name: project_name_clone.clone(),
                        message: line,
                        stream: "stderr".to_string(),
                    };
                    let _ = window_clone.emit("process_log", &log_msg);
                }
            });
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("命令执行失败: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("命令退出码 {}", status.code().unwrap_or(-1)))
        }
    }

    /// 停止项目
    pub async fn stop_project(&self, process_id: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().await;

        if let Some(handle) = processes.remove(process_id) {
            #[cfg(target_os = "windows")]
            {
                // Windows 使用 taskkill 来杀死整个进程树
                let pid = handle.child.id();
                let mut kill_command = Command::new("taskkill");
                kill_command.args(&["/PID", &pid.to_string(), "/T", "/F"]);

                // 隐藏 taskkill 窗口
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                kill_command.creation_flags(CREATE_NO_WINDOW);

                kill_command
                    .spawn()
                    .map_err(|e| format!("停止进程失败: {}", e))?;
            }

            #[cfg(not(target_os = "windows"))]
            {
                // macOS/Linux: 递归杀死整个进程树
                let pid = handle.child.id();
                kill_process_tree(pid)?;
            }

            Ok(())
        } else {
            Err("进程不存在".to_string())
        }
    }

    /// 获取所有运行中的进程
    pub async fn get_all_processes(&self) -> Vec<String> {
        let processes = self.processes.lock().await;
        processes.keys().cloned().collect()
    }

    /// 检查进程是否在运行
    pub async fn is_running(&self, process_id: &str) -> bool {
        let processes = self.processes.lock().await;
        processes.contains_key(process_id)
    }

    /// 停止所有进程
    pub async fn stop_all(&self) -> Result<(), String> {
        let process_ids: Vec<String> = {
            let processes = self.processes.lock().await;
            processes.keys().cloned().collect()
        };

        for process_id in process_ids {
            self.stop_project(&process_id).await?;
        }

        Ok(())
    }
}
