use crate::models::ProjectInfo;
use crate::services::config_parser::ConfigParser;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ProjectScanner;

impl ProjectScanner {
    /// 递归扫描目录查找 Zebras 项目
    /// max_depth: 最大递归深度
    pub fn scan_directory(root_path: &Path, max_depth: usize) -> Vec<ProjectInfo> {
        let mut projects = Vec::new();
        let skip_paths: RefCell<Vec<PathBuf>> = RefCell::new(Vec::new());

        for entry in WalkDir::new(root_path)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                let path = e.path();

                // 跳过 node_modules 目录
                if path.components().any(|c| c.as_os_str() == "node_modules") {
                    return false;
                }

                // 跳过已识别项目的子目录
                let skip_list = skip_paths.borrow();
                for skip_path in skip_list.iter() {
                    if path.starts_with(skip_path) && path != *skip_path {
                        return false;
                    }
                }

                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // 只检查目录
            if !path.is_dir() {
                continue;
            }

            // 快速检查：只有包含配置文件的目录才尝试解析
            let has_config = path.join("zebras.config.ts").exists()
                || path.join("zebra.json").exists();

            if !has_config {
                continue;
            }

            // 尝试解析项目配置
            match ConfigParser::parse_project(path) {
                Ok(project) => {
                    // 记录这个项目路径，避免扫描其子目录
                    skip_paths.borrow_mut().push(path.to_path_buf());
                    projects.push(project);
                }
                Err(_) => {
                    // 配置文件存在但解析失败，跳过
                    continue;
                }
            }
        }

        projects
    }

    /// 扫描指定的文件夹列表
    pub fn scan_folders(folders: &[String], max_depth: usize) -> Vec<ProjectInfo> {
        let mut all_projects = Vec::new();

        for folder in folders {
            let path = Path::new(folder);
            if path.exists() && path.is_dir() {
                let projects = Self::scan_directory(path, max_depth);
                all_projects.extend(projects);
            }
        }

        all_projects
    }

    /// 检查单个路径是否是 Zebras 项目
    pub fn is_zebras_project(path: &Path) -> bool {
        ConfigParser::parse_project(path).is_ok()
    }

    /// 重新扫描单个项目
    pub fn rescan_project(path: &Path) -> Result<ProjectInfo, String> {
        ConfigParser::parse_project(path).map_err(|e| format!("扫描项目失败: {:?}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_scan_directory() {
        // 这个测试需要实际的 Zebras 项目目录才能运行
        // 这里只是示例
        let path = PathBuf::from(".");
        let projects = ProjectScanner::scan_directory(&path, 2);
        // 断言会根据实际情况而定
    }
}
