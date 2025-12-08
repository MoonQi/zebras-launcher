use crate::models::{ProcessInfo, ProcessStatus};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};
use chrono::Utc;
use serde::Serialize;
use once_cell::sync::Lazy;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Clone, Serialize)]
pub struct LogMessage {
    pub process_id: String,
    pub project_name: String,
    pub message: String,
    pub stream: String, // "stdout" or "stderr"
}

/// 缓存用户终端的完整 PATH 环境变量 (macOS/Linux)
/// 这样可以确保获取到 nvm、fnm、pnpm、npm global 等所有路径
#[cfg(not(target_os = "windows"))]
static USER_PATH: Lazy<String> = Lazy::new(|| {
    get_user_shell_path().unwrap_or_else(|_| std::env::var("PATH").unwrap_or_default())
});

/// 从用户的 login shell 获取完整的 PATH 环境变量
#[cfg(not(target_os = "windows"))]
fn get_user_shell_path() -> Result<String, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    
    // 通过 login interactive shell 获取完整 PATH
    // -l = login shell (读取 .zprofile/.bash_profile)
    // -i = interactive shell (读取 .zshrc/.bashrc)
    // -c = 执行命令
    let output = Command::new(&shell)
        .args(&["-l", "-i", "-c", "echo $PATH"])
        .output()
        .map_err(|e| format!("获取用户 PATH 失败: {}", e))?;
    
    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            println!("[ProcessManager] 获取到用户 PATH: {}", path);
            return Ok(path);
        }
    }
    
    Err("无法获取用户 PATH".to_string())
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

            command.spawn().map_err(|e| format!("启动项目失败: {}", e))?
        };

        #[cfg(not(target_os = "windows"))]
        let mut child = {
            // macOS/Linux: 使用缓存的用户 PATH 环境变量
            // 这个 PATH 是从用户的 login shell 获取的，包含了所有全局命令路径
            let mut command = Command::new("npm");
            command
                .args(&["run", "start"])
                .current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("PATH", &*USER_PATH);  // 使用用户终端的完整 PATH

            command.spawn().map_err(|e| format!("启动项目失败: {}", e))?
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
                // macOS/Linux: 递归杀死整个进程树
                let pid = handle.child.id();
                Self::kill_process_tree(pid);
            }

            Ok(())
        } else {
            Err("进程不存在".to_string())
        }
    }

    /// 递归获取所有子进程 ID (macOS/Linux)
    #[cfg(not(target_os = "windows"))]
    fn get_child_pids(pid: u32) -> Vec<u32> {
        let output = Command::new("pgrep")
            .args(&["-P", &pid.to_string()])
            .output();

        match output {
            Ok(output) => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| line.trim().parse::<u32>().ok())
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// 递归杀死进程树 (macOS/Linux)
    #[cfg(not(target_os = "windows"))]
    fn kill_process_tree(pid: u32) {
        // 先递归收集所有子进程（深度优先）
        let children = Self::get_child_pids(pid);
        for child_pid in children {
            Self::kill_process_tree(child_pid);
        }

        // 杀死当前进程：先 SIGTERM，再 SIGKILL
        let _ = Command::new("kill")
            .args(&["-TERM", &pid.to_string()])
            .output(); // 使用 output() 等待命令完成

        // 短暂等待进程优雅退出
        std::thread::sleep(std::time::Duration::from_millis(50));

        // 强制杀死（如果还在运行）
        let _ = Command::new("kill")
            .args(&["-9", &pid.to_string()])
            .output();
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
