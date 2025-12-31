use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(not(target_os = "windows"))]
fn get_child_pids(pid: u32) -> Vec<u32> {
    let output = Command::new("pgrep")
        .args(&["-P", &pid.to_string()])
        .output();

    match output {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// 递归杀死进程树
pub fn kill_process_tree(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut kill_command = Command::new("taskkill");
        kill_command.args(&["/PID", &pid.to_string(), "/T", "/F"]);

        const CREATE_NO_WINDOW: u32 = 0x08000000;
        kill_command.creation_flags(CREATE_NO_WINDOW);

        kill_command
            .spawn()
            .map_err(|e| format!("停止进程失败: {}", e))?;

        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 先递归收集所有子进程（深度优先）
        let children = get_child_pids(pid);
        for child_pid in children {
            let _ = kill_process_tree(child_pid);
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

        return Ok(());
    }
}
