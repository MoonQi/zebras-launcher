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

/// 缓存用户的完整 PATH 环境变量 (macOS/Linux)
/// 通过读取 shell 配置文件来获取，避免启动 interactive shell 触发授权提示
#[cfg(not(target_os = "windows"))]
static USER_PATH: Lazy<String> = Lazy::new(|| {
    get_user_shell_path().unwrap_or_else(|e| {
        println!("[ProcessManager] 无法获取用户 PATH: {}, 使用系统 PATH", e);
        std::env::var("PATH").unwrap_or_default()
    })
});

/// 从用户的 shell 获取完整的 PATH 环境变量
/// 优化：使用非交互式 login shell，避免触发 macOS 授权提示
#[cfg(not(target_os = "windows"))]
fn get_user_shell_path() -> Result<String, String> {
    // 方法 1: 尝试使用非交互式 login shell (只用 -l，不用 -i)
    // 这样只会读取 .zprofile/.bash_profile，避免 .zshrc 中可能触发交互的内容
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    
    let output = Command::new(&shell)
        .args(&["-l", "-c", "echo $PATH"])
        .stdin(Stdio::null())  // 明确关闭 stdin，确保非交互
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && path.contains("/") {
                println!("[ProcessManager] 通过 login shell 获取到 PATH");
                return Ok(path);
            }
        }
    }
    
    // 方法 2: 直接构建常用的 PATH 路径
    // 这是一个更可靠的 fallback，包含大多数 Node.js 版本管理器的路径
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let system_path = std::env::var("PATH").unwrap_or_default();
    
    let common_paths = vec![
        format!("{}/bin", home),
        format!("{}/.local/bin", home),
        // nvm
        format!("{}/.nvm/versions/node/*/bin", home),
        // fnm
        format!("{}/.fnm/current/bin", home),
        format!("{}/Library/Application Support/fnm/current/bin", home),
        // Homebrew (Apple Silicon & Intel)
        "/opt/homebrew/bin".to_string(),
        "/opt/homebrew/sbin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/local/sbin".to_string(),
        // pnpm
        format!("{}/Library/pnpm", home),
        format!("{}/.pnpm-global/bin", home),
        // npm global
        format!("{}/.npm-global/bin", home),
        // volta
        format!("{}/.volta/bin", home),
        // 系统路径
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];
    
    // 过滤存在的路径并与系统 PATH 合并
    let mut paths: Vec<String> = common_paths
        .into_iter()
        .filter(|p| !p.contains("*") && std::path::Path::new(p).exists())
        .collect();
    
    // 特殊处理 nvm（需要查找实际的 node 版本目录）
    let nvm_dir = format!("{}/.nvm/versions/node", home);
    if std::path::Path::new(&nvm_dir).exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    paths.insert(0, bin_path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    // 将系统 PATH 中的路径添加到末尾（去重）
    for p in system_path.split(':') {
        if !p.is_empty() && !paths.contains(&p.to_string()) {
            paths.push(p.to_string());
        }
    }
    
    let final_path = paths.join(":");
    println!("[ProcessManager] 使用构建的 PATH 路径");
    Ok(final_path)
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
