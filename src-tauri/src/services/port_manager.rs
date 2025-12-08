use crate::models::{ProjectInfo, PortChange};
use crate::utils::port_checker::is_port_available;
use crate::services::config_parser::ConfigParser;
use std::collections::HashSet;

pub struct PortManager {
    used_ports: HashSet<u16>,
    range_start: u16,
    range_end: u16,
}

impl PortManager {
    pub fn new(range_start: u16, range_end: u16) -> Self {
        Self {
            used_ports: HashSet::new(),
            range_start,
            range_end,
        }
    }

    /// 解决端口冲突，返回需要修改的端口变更列表
    /// global_used_ports: 全局已使用的端口（来自所有工作区）
    pub fn resolve_conflicts(
        &mut self,
        projects: &mut Vec<ProjectInfo>,
        global_used_ports: &HashSet<u16>,
    ) -> Result<Vec<PortChange>, String> {
        let mut changes = Vec::new();
        self.used_ports.clear();

        // 初始化已使用端口集合，包含全局端口
        self.used_ports.extend(global_used_ports);

        for project in projects.iter_mut() {
            let requested_port = project.port;

            // 检查端口是否可用
            if !is_port_available(requested_port) || self.used_ports.contains(&requested_port) {
                // 查找下一个可用端口
                match self.find_next_port(requested_port) {
                    Some(new_port) => {
                        changes.push(PortChange {
                            project_name: project.name.clone(),
                            old_port: requested_port,
                            new_port,
                        });

                        project.port = new_port;
                        self.used_ports.insert(new_port);
                    }
                    None => {
                        return Err(format!(
                            "无法为项目 {} 找到可用端口",
                            project.name
                        ));
                    }
                }
            } else {
                self.used_ports.insert(requested_port);
            }
        }

        Ok(changes)
    }

    /// 顺序递增查找下一个可用端口
    fn find_next_port(&self, start: u16) -> Option<u16> {
        let mut port = start;

        while port <= self.range_end {
            if is_port_available(port) && !self.used_ports.contains(&port) {
                return Some(port);
            }
            port += 1;
        }

        None
    }

    /// 应用端口变更到本地配置文件
    pub fn apply_port_changes(changes: &[PortChange], projects: &[ProjectInfo]) -> Result<(), String> {
        for change in changes {
            // 找到对应的项目
            if let Some(project) = projects.iter().find(|p| p.name == change.project_name) {
                ConfigParser::update_port(project, change.new_port)
                    .map_err(|e| format!("更新端口配置失败: {:?}", e))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ZebrasVersion;
    use std::path::PathBuf;

    #[test]
    fn test_port_manager_sequential() {
        let mut manager = PortManager::new(8000, 9000);

        let mut projects = vec![
            ProjectInfo {
                id: "1".to_string(),
                path: PathBuf::from("/test1"),
                version: ZebrasVersion::V3,
                platform: "web".to_string(),
                type_: "app".to_string(),
                name: "test1".to_string(),
                domain: None,
                port: 59000, // 使用一个测试端口
                framework: None,
                is_valid: true,
                last_scanned: chrono::Utc::now(),
                error: None,
                debug: None,
                enabled: None,
            },
        ];

        let global_ports = HashSet::new();
        let result = manager.resolve_conflicts(&mut projects, &global_ports);
        assert!(result.is_ok());
    }
}
