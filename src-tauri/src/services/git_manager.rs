use crate::models::{GitPullResult, GitStatus};
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(not(target_os = "windows"))]
use crate::utils::USER_PATH;

pub struct GitManager;

impl GitManager {
    pub fn new() -> Self {
        Self
    }

    pub fn is_git_repo(path: &str) -> bool {
        Path::new(path).join(".git").exists()
    }

    fn run_git(args: &[&str], cwd: &str) -> Result<(i32, String, String), String> {
        let mut cmd = Command::new("git");
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(not(target_os = "windows"))]
        {
            cmd.env("PATH", &*USER_PATH);
        }

        let output = cmd.output().map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                "GIT_NOT_INSTALLED".to_string()
            } else {
                format!("执行 git 失败: {}", e)
            }
        })?;

        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((code, stdout, stderr))
    }

    fn run_git_checked(args: &[&str], cwd: &str) -> Result<String, String> {
        let (code, stdout, stderr) = Self::run_git(args, cwd)?;
        if code == 0 {
            Ok(stdout.trim().to_string())
        } else {
            let err = stderr.trim();
            if err.is_empty() {
                Err(format!("git {:?} 失败，退出码 {}", args, code))
            } else {
                Err(err.to_string())
            }
        }
    }

    fn get_status_sync(path: &str) -> Result<GitStatus, String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        let branch_raw =
            Self::run_git_checked(&["rev-parse", "--abbrev-ref", "HEAD"], path).unwrap_or_default();
        let branch = match branch_raw.as_str() {
            "" => None,
            "HEAD" => None,
            _ => Some(branch_raw),
        };

        let porcelain = Self::run_git_checked(&["status", "--porcelain"], path).unwrap_or_default();
        let uncommitted_count = porcelain
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count() as u32;

        let mut status = GitStatus {
            branch,
            has_remote: false,
            uncommitted_count,
            ahead_count: 0,
            behind_count: 0,
        };

        match Self::run_git_checked(
            &["rev-list", "--count", "--left-right", "@{u}...HEAD"],
            path,
        ) {
            Ok(out) => {
                let parts: Vec<&str> = out.split_whitespace().collect();
                if parts.len() >= 2 {
                    status.ahead_count = parts[0].parse::<u32>().unwrap_or(0);
                    status.behind_count = parts[1].parse::<u32>().unwrap_or(0);
                }
                status.has_remote = true;
            }
            Err(_) => {
                status.has_remote = false;
                status.ahead_count = 0;
                status.behind_count = 0;
            }
        }

        Ok(status)
    }

    fn fetch_sync(path: &str) -> Result<(), String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        let _ = Self::run_git_checked(&["fetch", "--quiet"], path)?;
        Ok(())
    }

    fn pull_sync(path: &str) -> Result<GitPullResult, String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        let before = Self::get_status_sync(path)?;
        if before.uncommitted_count > 0 {
            return Err("当前存在未提交更改，已禁用 Pull".to_string());
        }

        let result = Self::run_git(&["pull", "--ff-only"], path)?;
        let ok = result.0 == 0;

        let after = Self::get_status_sync(path)?;
        let message = if ok {
            "Pull 成功".to_string()
        } else {
            let err = result.2.trim();
            if err.is_empty() {
                format!("Pull 失败，退出码 {}", result.0)
            } else {
                err.to_string()
            }
        };

        Ok(GitPullResult {
            success: ok,
            message,
            status: after,
        })
    }

    pub async fn get_status(&self, path: String) -> Result<GitStatus, String> {
        tokio::task::spawn_blocking(move || Self::get_status_sync(&path))
            .await
            .map_err(|e| format!("任务失败: {}", e))?
    }

    pub async fn fetch(&self, path: String) -> Result<GitStatus, String> {
        tokio::task::spawn_blocking(move || {
            Self::fetch_sync(&path)?;
            Self::get_status_sync(&path)
        })
        .await
        .map_err(|e| format!("任务失败: {}", e))?
    }

    pub async fn pull(&self, path: String) -> Result<GitPullResult, String> {
        tokio::task::spawn_blocking(move || Self::pull_sync(&path))
            .await
            .map_err(|e| format!("任务失败: {}", e))?
    }
}
