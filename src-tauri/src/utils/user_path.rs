#![cfg(not(target_os = "windows"))]

use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};

use once_cell::sync::Lazy;

/// 缓存用户的完整 PATH 环境变量 (macOS/Linux)
/// 优先合并 interactive/login shell 与常见 Node 管理器路径，尽量贴近用户终端环境
pub static USER_PATH: Lazy<String> = Lazy::new(|| {
    get_user_shell_path().unwrap_or_else(|e| {
        println!("[user_path] 无法获取用户 PATH: {}, 使用系统 PATH", e);
        std::env::var("PATH").unwrap_or_default()
    })
});

/// 从用户的 shell 获取完整的 PATH 环境变量
fn get_user_shell_path() -> Result<String, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users".to_string());
    let system_path = std::env::var("PATH").unwrap_or_default();
    let mut path_candidates = Vec::new();

    if let Some(path) = read_path_from_shell(&shell, &["-i", "-c", "printf '%s\\n' \"$PATH\""]) {
        println!("[user_path] 通过 interactive shell 获取到 PATH");
        path_candidates.push(path);
    }

    if let Some(path) = read_path_from_shell(&shell, &["-l", "-c", "printf '%s\\n' \"$PATH\""]) {
        println!("[user_path] 通过 login shell 获取到 PATH");
        path_candidates.push(path);
    }

    let fallback_entries = build_fallback_path_entries(&home, &system_path);
    if !fallback_entries.is_empty() {
        println!("[user_path] 使用常见 Node 管理器路径作为补充");
        path_candidates.push(fallback_entries.join(":"));
    }

    let final_path = merge_path_candidates(&path_candidates);
    if final_path.is_empty() {
        return Err("未能构建可用 PATH".to_string());
    }

    Ok(final_path)
}

fn read_path_from_shell(shell: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(shell)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_path_output(&String::from_utf8_lossy(&output.stdout))
}

fn build_fallback_path_entries(home: &str, system_path: &str) -> Vec<String> {
    let common_paths = vec![
        format!("{}/bin", home),
        format!("{}/.local/bin", home),
        format!("{}/.local/share/pnpm", home),
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
    ];

    let mut paths: Vec<String> = common_paths
        .into_iter()
        .filter(|p| !p.contains('*') && Path::new(p).exists())
        .collect();

    let nvm_dir = format!("{}/.nvm/versions/node", home);
    if Path::new(&nvm_dir).exists() {
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                let bin_path = entry.path().join("bin");
                if bin_path.exists() {
                    paths.insert(0, bin_path.to_string_lossy().to_string());
                }
            }
        }
    }

    for p in system_path.split(':') {
        let trimmed = p.trim();
        if trimmed.starts_with('/') && !paths.iter().any(|existing| existing == trimmed) {
            paths.push(trimmed.to_string());
        }
    }

    paths
}

fn parse_path_output(output: &str) -> Option<String> {
    output.lines().rev().find_map(normalize_path_line)
}

fn normalize_path_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed.strip_prefix("PATH=").unwrap_or(trimmed);
    let entries: Vec<&str> = candidate
        .split(':')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();

    if entries.is_empty() || entries.iter().any(|entry| !entry.starts_with('/')) {
        return None;
    }

    Some(entries.join(":"))
}

fn merge_path_candidates(candidates: &[String]) -> String {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for candidate in candidates {
        if let Some(path_line) = parse_path_output(candidate) {
            for entry in path_line.split(':') {
                let normalized = entry.trim();
                if normalized.is_empty() {
                    continue;
                }

                if seen.insert(normalized.to_string()) {
                    merged.push(normalized.to_string());
                }
            }
        }
    }

    merged.join(":")
}

pub fn resolve_program_in_user_path(program: &str) -> Option<String> {
    for dir in USER_PATH.split(':') {
        let candidate = Path::new(dir).join(program);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_path_output_uses_last_valid_path_line() {
        let output = r#"
loading shell profile
PATH=/usr/local/bin:/usr/bin:/bin
"#;

        assert_eq!(
            parse_path_output(output).as_deref(),
            Some("/usr/local/bin:/usr/bin:/bin")
        );
    }

    #[test]
    fn merge_path_candidates_deduplicates_and_preserves_order() {
        let merged = merge_path_candidates(&[
            "/opt/homebrew/bin:/usr/bin".to_string(),
            "PATH=/Users/test/Library/pnpm:/usr/bin".to_string(),
            "/opt/homebrew/bin:/Users/test/.volta/bin".to_string(),
        ]);

        assert_eq!(
            merged,
            "/opt/homebrew/bin:/usr/bin:/Users/test/Library/pnpm:/Users/test/.volta/bin"
        );
    }
}
