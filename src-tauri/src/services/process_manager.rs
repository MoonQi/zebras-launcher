use crate::models::{ProcessInfo, ProcessStatus};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};
use chrono::Utc;
use serde::Serialize;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Clone, Serialize)]
pub struct LogMessage {
    pub process_id: String,
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
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            window,
        }
    }

    /// 查找 npm 命令路径
    fn find_npm_command() -> Result<String, String> {
        #[cfg(target_os = "windows")]
        {
            return Ok("npm.cmd".to_string());
        }

        #[cfg(not(target_os = "windows"))]
        {
            // macOS/Linux: 尝试多个可能的 npm 路径
            let possible_paths = [
                "/opt/homebrew/bin/npm",      // macOS Apple Silicon (Homebrew)
                "/usr/local/bin/npm",         // macOS Intel (Homebrew) / Linux
                "/usr/bin/npm",               // Linux 系统安装
                "npm",                        // 回退到 PATH 查找
            ];

            for path in possible_paths {
                if path == "npm" {
                    return Ok(path.to_string());
                }
                if std::path::Path::new(path).exists() {
                    return Ok(path.to_string());
                }
            }

            Ok("npm".to_string())
        }
    }

    /// 启动项目
    pub async fn start_project(
        &self,
        project_id: String,
        project_name: String,
        project_path: String,
    ) -> Result<ProcessInfo, String> {
        let process_id = uuid::Uuid::new_v4().to_string();

        // 跨平台 npm 命令
        let npm_cmd = Self::find_npm_command()?;

        // 创建命令
        let mut command = Command::new(&npm_cmd);
        command
            .args(&["run", "start"])
            .current_dir(&project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // macOS/Linux: 设置完整的 PATH 环境变量
        #[cfg(not(target_os = "windows"))]
        {
            let path = std::env::var("PATH").unwrap_or_default();
            let extra_paths = "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin";
            let new_path = format!("{}:{}", extra_paths, path);
            command.env("PATH", new_path);
        }

        // Windows: 隐藏控制台窗口
        #[cfg(target_os = "windows")]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        // 启动 npm run start
        let mut child = command
            .spawn()
            .map_err(|e| format!("启动项目失败: {}", e))?;

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

        self.processes.lock().await.insert(process_id.clone(), handle);

        // 启动日志流任务
        if let Some(stdout) = stdout {
            let process_id_clone = process_id.clone();
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(tokio::process::ChildStdout::from_std(stdout).unwrap());
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
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
            let project_name_clone = project_name.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(tokio::process::ChildStderr::from_std(stderr).unwrap());
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = LogMessage {
                        process_id: process_id_clone.clone(),
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
                // macOS/Linux: 使用 pkill 杀死整个进程组
                let pid = handle.child.id();
                // 先尝试发送 SIGTERM 给进程组
                let _ = Command::new("pkill")
                    .args(&["-TERM", "-P", &pid.to_string()])
                    .spawn();
                // 等待一小段时间后强制杀死
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = Command::new("pkill")
                    .args(&["-KILL", "-P", &pid.to_string()])
                    .spawn();
                // 最后杀死父进程
                let _ = Command::new("kill")
                    .args(&["-9", &pid.to_string()])
                    .spawn();
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
