#![cfg(not(target_os = "windows"))]

use std::path::Path;
use std::process::{Command, Stdio};

use once_cell::sync::Lazy;

/// 缓存用户的完整 PATH 环境变量 (macOS/Linux)
/// 通过读取 shell 配置文件来获取，避免启动 interactive shell 触发授权提示
pub static USER_PATH: Lazy<String> = Lazy::new(|| {
    get_user_shell_path().unwrap_or_else(|e| {
        println!("[user_path] 无法获取用户 PATH: {}, 使用系统 PATH", e);
        std::env::var("PATH").unwrap_or_default()
    })
});

/// 从用户的 shell 获取完整的 PATH 环境变量
/// 优化：使用非交互式 login shell，避免触发 macOS 授权提示
fn get_user_shell_path() -> Result<String, String> {
    // 方法 1: 尝试使用非交互式 login shell (只用 -l，不用 -i)
    // 这样只会读取 .zprofile/.bash_profile，避免 .zshrc 中可能触发交互的内容
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

    let output = Command::new(&shell)
        .args(&["-l", "-c", "echo $PATH"])
        .stdin(Stdio::null()) // 明确关闭 stdin，确保非交互
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && path.contains('/') {
                println!("[user_path] 通过 login shell 获取到 PATH");
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
        // 系统路径
        "/usr/bin".to_string(),
        "/bin".to_string(),
        "/usr/sbin".to_string(),
        "/sbin".to_string(),
    ];

    // 过滤存在的路径并与系统 PATH 合并
    let mut paths: Vec<String> = common_paths
        .into_iter()
        .filter(|p| !p.contains('*') && Path::new(p).exists())
        .collect();

    // 特殊处理 nvm（需要查找实际的 node 版本目录）
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

    // 将系统 PATH 中的路径添加到末尾（去重）
    for p in system_path.split(':') {
        if !p.is_empty() && !paths.contains(&p.to_string()) {
            paths.push(p.to_string());
        }
    }

    let final_path = paths.join(":");
    println!("[user_path] 使用构建的 PATH 路径");
    Ok(final_path)
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
