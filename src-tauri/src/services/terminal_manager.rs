use crate::models::{TerminalSession, TerminalStatus};
use crate::utils::kill_process_tree;
use serde::Serialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;

#[cfg(not(target_os = "windows"))]
use crate::utils::USER_PATH;

#[derive(Clone, Serialize)]
pub struct TerminalLogMessage {
    pub session_id: String,
    pub project_id: String,
    pub message: String,
    pub stream: String, // "stdout" or "stderr"
}

pub struct TerminalManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
    window: tauri::Window,
}

impl TerminalManager {
    pub fn new(window: tauri::Window) -> Self {
        #[cfg(not(target_os = "windows"))]
        {
            let _ = &*USER_PATH;
        }

        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            window,
        }
    }

    pub async fn create_session(&self, project_id: String) -> Result<TerminalSession, String> {
        let mut sessions = self.sessions.lock().await;
        let count = sessions
            .values()
            .filter(|s| s.project_id == project_id)
            .count();
        if count >= 3 {
            return Err("每个项目最多可打开 3 个终端".to_string());
        }

        let session = TerminalSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            project_id,
            command: None,
            status: TerminalStatus::Idle,
            pid: None,
        };

        sessions.insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    pub async fn get_sessions(&self, project_id: String) -> Vec<TerminalSession> {
        let sessions = self.sessions.lock().await;
        sessions
            .values()
            .filter(|s| s.project_id == project_id)
            .cloned()
            .collect()
    }

    pub async fn run_command(
        &self,
        session_id: String,
        project_path: String,
        command: String,
    ) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("命令不能为空".to_string());
        }

        let project_id = {
            let mut sessions = self.sessions.lock().await;
            let session = sessions
                .get_mut(&session_id)
                .ok_or_else(|| "终端不存在".to_string())?;

            if session.status == TerminalStatus::Running {
                return Err("该终端正在运行中".to_string());
            }

            session.command = Some(command.clone());
            session.status = TerminalStatus::Running;
            session.pid = None;
            session.project_id.clone()
        };

        let mut cmd = {
            #[cfg(target_os = "windows")]
            let mut c = {
                let mut c = TokioCommand::new("cmd");
                c.args(&["/C", &command]);
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                c.creation_flags(CREATE_NO_WINDOW);
                c
            };

            #[cfg(not(target_os = "windows"))]
            let mut c = {
                let mut c = TokioCommand::new("sh");
                c.args(&["-c", &command]);
                c.env("PATH", &*USER_PATH);
                c
            };

            c.current_dir(&project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            c
        };

        let mut child = cmd.spawn().map_err(|e| format!("执行命令失败: {}", e))?;

        let pid = child.id();
        {
            let mut sessions = self.sessions.lock().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.pid = pid;
            }
        }

        if let Some(stdout) = child.stdout.take() {
            let session_id_clone = session_id.clone();
            let project_id_clone = project_id.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = TerminalLogMessage {
                        session_id: session_id_clone.clone(),
                        project_id: project_id_clone.clone(),
                        message: line,
                        stream: "stdout".to_string(),
                    };
                    let _ = window_clone.emit("terminal_log", &log_msg);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let session_id_clone = session_id.clone();
            let project_id_clone = project_id.clone();
            let window_clone = self.window.clone();

            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let log_msg = TerminalLogMessage {
                        session_id: session_id_clone.clone(),
                        project_id: project_id_clone.clone(),
                        message: line,
                        stream: "stderr".to_string(),
                    };
                    let _ = window_clone.emit("terminal_log", &log_msg);
                }
            });
        }

        let sessions_clone = self.sessions.clone();
        let window_clone = self.window.clone();
        tokio::spawn(async move {
            let status = child.wait().await;

            let (new_status, exit_code) = match status {
                Ok(exit) if exit.success() => (TerminalStatus::Completed, exit.code()),
                Ok(exit) => (TerminalStatus::Error, exit.code()),
                Err(_) => (TerminalStatus::Error, None),
            };

            let mut sessions = sessions_clone.lock().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = new_status;
                session.pid = None;
            }

            let msg = match exit_code {
                Some(code) => format!("[exit] code={}", code),
                None => "[exit]".to_string(),
            };
            let _ = window_clone.emit(
                "terminal_log",
                &TerminalLogMessage {
                    session_id,
                    project_id,
                    message: msg,
                    stream: "stdout".to_string(),
                },
            );
        });

        Ok(())
    }

    pub async fn kill_session(&self, session_id: &str) -> Result<(), String> {
        let pid = {
            let sessions = self.sessions.lock().await;
            let session = sessions
                .get(session_id)
                .ok_or_else(|| "终端不存在".to_string())?;
            session
                .pid
                .ok_or_else(|| "该终端当前没有运行中的进程".to_string())?
        };

        kill_process_tree(pid)?;

        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = TerminalStatus::Error;
            session.pid = None;
        }

        Ok(())
    }

    pub async fn close_session(&self, session_id: &str) -> Result<(), String> {
        let pid = {
            let sessions = self.sessions.lock().await;
            sessions.get(session_id).and_then(|s| s.pid)
        };

        if let Some(pid) = pid {
            let _ = kill_process_tree(pid);
        }

        let mut sessions = self.sessions.lock().await;
        if sessions.remove(session_id).is_some() {
            Ok(())
        } else {
            Err("终端不存在".to_string())
        }
    }

    pub async fn stop_all(&self) -> Result<(), String> {
        let pids: Vec<u32> = {
            let sessions = self.sessions.lock().await;
            sessions.values().filter_map(|s| s.pid).collect()
        };

        for pid in pids {
            let _ = kill_process_tree(pid);
        }

        self.sessions.lock().await.clear();
        Ok(())
    }
}
