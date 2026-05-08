use crate::models::{GitBranch, GitPullResult, GitStatus, GitSwitchResult};
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

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

        // Ensure Git never blocks on interactive terminal prompts in a GUI app.
        cmd.env("GIT_TERMINAL_PROMPT", "0");
        cmd.env("GCM_INTERACTIVE", "never");

        #[cfg(target_os = "windows")]
        {
            // Prevent spawning a new console window in packaged (GUI) builds.
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

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

    fn is_switch_unsupported(stderr: &str) -> bool {
        stderr.contains("not a git command") || stderr.contains("unknown subcommand")
    }

    fn parse_local_branches(output: &str) -> Vec<GitBranch> {
        output
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }

                let mut parts = trimmed.split('\t');
                let name = parts.next()?.trim().to_string();
                if name.is_empty() {
                    return None;
                }
                let head = parts.next().unwrap_or("").trim();
                let upstream = parts
                    .next()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string());

                Some(GitBranch {
                    name,
                    is_remote: false,
                    is_current: head == "*",
                    upstream,
                })
            })
            .collect()
    }

    fn parse_remote_branches(output: &str) -> Vec<GitBranch> {
        output
            .lines()
            .filter_map(|line| {
                let name = line.trim();
                if name.is_empty() || name.ends_with("/HEAD") {
                    return None;
                }

                Some(GitBranch {
                    name: name.to_string(),
                    is_remote: true,
                    is_current: false,
                    upstream: None,
                })
            })
            .collect()
    }

    fn derive_local_branch_name(remote_branch: &str) -> &str {
        remote_branch
            .split_once('/')
            .map(|(_, branch)| branch)
            .unwrap_or(remote_branch)
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

    fn list_branches_sync(path: &str, fetch_first: bool) -> Result<Vec<GitBranch>, String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        if fetch_first {
            let _ = Self::fetch_sync(path);
        }

        let local_output = Self::run_git_checked(
            &[
                "for-each-ref",
                "--sort=refname",
                "--format=%(refname:short)\t%(HEAD)\t%(upstream:short)",
                "refs/heads",
            ],
            path,
        )?;
        let remote_output = Self::run_git_checked(
            &[
                "for-each-ref",
                "--sort=refname",
                "--format=%(refname:short)",
                "refs/remotes",
            ],
            path,
        )?;

        let mut branches = Self::parse_local_branches(&local_output);
        branches.extend(Self::parse_remote_branches(&remote_output));
        Ok(branches)
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

    fn switch_local_branch_sync(path: &str, branch_name: &str) -> Result<GitSwitchResult, String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        let switch_result = Self::run_git(&["switch", branch_name], path)?;
        let ok = if switch_result.0 == 0 {
            true
        } else if Self::is_switch_unsupported(&switch_result.2) {
            let checkout_result = Self::run_git(&["checkout", branch_name], path)?;
            if checkout_result.0 != 0 {
                let err = checkout_result.2.trim();
                return Err(if err.is_empty() {
                    format!("切换分支失败，退出码 {}", checkout_result.0)
                } else {
                    err.to_string()
                });
            }
            true
        } else {
            false
        };

        if !ok {
            let err = switch_result.2.trim();
            return Err(if err.is_empty() {
                format!("切换分支失败，退出码 {}", switch_result.0)
            } else {
                err.to_string()
            });
        }

        let status = Self::get_status_sync(path)?;
        Ok(GitSwitchResult {
            message: format!(
                "已切换到 {}",
                status
                    .branch
                    .clone()
                    .unwrap_or_else(|| branch_name.to_string())
            ),
            status,
        })
    }

    fn switch_remote_branch_sync(
        path: &str,
        remote_branch: &str,
    ) -> Result<GitSwitchResult, String> {
        if !Self::is_git_repo(path) {
            return Err("NOT_GIT_REPO".to_string());
        }

        let local_branch = Self::derive_local_branch_name(remote_branch).to_string();
        let local_ref = format!("refs/heads/{}", local_branch);
        let local_exists =
            Self::run_git(&["show-ref", "--verify", "--quiet", &local_ref], path)?.0 == 0;

        if local_exists {
            return Self::switch_local_branch_sync(path, &local_branch);
        }

        let switch_result = Self::run_git(&["switch", "--track", remote_branch], path)?;
        let ok = if switch_result.0 == 0 {
            true
        } else if Self::is_switch_unsupported(&switch_result.2) {
            let checkout_result = Self::run_git(&["checkout", "--track", remote_branch], path)?;
            if checkout_result.0 != 0 {
                let err = checkout_result.2.trim();
                return Err(if err.is_empty() {
                    format!("切换远端分支失败，退出码 {}", checkout_result.0)
                } else {
                    err.to_string()
                });
            }
            true
        } else {
            false
        };

        if !ok {
            let err = switch_result.2.trim();
            return Err(if err.is_empty() {
                format!("切换远端分支失败，退出码 {}", switch_result.0)
            } else {
                err.to_string()
            });
        }

        let status = Self::get_status_sync(path)?;
        Ok(GitSwitchResult {
            message: format!("已切换到 {}", status.branch.clone().unwrap_or(local_branch)),
            status,
        })
    }

    pub async fn get_status(&self, path: String) -> Result<GitStatus, String> {
        tokio::task::spawn_blocking(move || Self::get_status_sync(&path))
            .await
            .map_err(|e| format!("任务失败: {}", e))?
    }

    pub async fn list_branches(
        &self,
        path: String,
        fetch_first: bool,
    ) -> Result<Vec<GitBranch>, String> {
        tokio::task::spawn_blocking(move || Self::list_branches_sync(&path, fetch_first))
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

    pub async fn switch_branch(
        &self,
        path: String,
        branch_name: String,
        is_remote: bool,
    ) -> Result<GitSwitchResult, String> {
        tokio::task::spawn_blocking(move || {
            if is_remote {
                Self::switch_remote_branch_sync(&path, &branch_name)
            } else {
                Self::switch_local_branch_sync(&path, &branch_name)
            }
        })
        .await
        .map_err(|e| format!("任务失败: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::GitManager;

    #[test]
    fn parse_local_branches_marks_current_and_upstream() {
        let branches = GitManager::parse_local_branches(
            "main\t*\torigin/main\nfeature/test\t \torigin/feature/test\n",
        );
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
        assert_eq!(branches[1].name, "feature/test");
        assert!(!branches[1].is_current);
    }

    #[test]
    fn parse_remote_branches_skips_head_pointer() {
        let branches =
            GitManager::parse_remote_branches("origin/HEAD\norigin/main\norigin/feature/test\n");
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "origin/main");
        assert!(branches.iter().all(|branch| branch.is_remote));
    }

    #[test]
    fn derive_local_branch_name_preserves_nested_branch_name() {
        assert_eq!(
            GitManager::derive_local_branch_name("origin/feature/test"),
            "feature/test"
        );
    }
}
