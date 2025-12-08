use crate::models::{ProjectInfo, ZebrasVersion};
use crate::utils::ts_parser;
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum ParseError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    MissingField(String),
    NotAZebrasProject,
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err)
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(err: serde_json::Error) -> Self {
        ParseError::JsonError(err)
    }
}

pub struct ConfigParser;

impl ConfigParser {
    /// 解析 Zebras v2 项目配置 (JSON)
    pub fn parse_v2_config(project_path: &Path) -> Result<ProjectInfo, ParseError> {
        // 读取 zebra.json
        let main_config_path = project_path.join("zebra.json");
        if !main_config_path.exists() {
            return Err(ParseError::NotAZebrasProject);
        }

        let main_json = fs::read_to_string(&main_config_path)?;
        let main: Value = serde_json::from_str(&main_json)?;

        // 读取 zebra.local.json (可选)
        let local_config_path = project_path.join("zebra.local.json");
        let local: Value = if local_config_path.exists() {
            let local_json = fs::read_to_string(&local_config_path)?;
            serde_json::from_str(&local_json)?
        } else {
            Value::Object(serde_json::Map::new())
        };

        // 将本地配置整体覆盖主配置
        let merged = Self::merge_json(&main, &local);

        // 提取字段（local > main）
        let platform = merged
            .get("platform")
            .and_then(|v| v.as_str())
            .unwrap_or("web")
            .to_string();

        let type_ = merged
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("app")
            .to_string();

        let name = merged
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ParseError::MissingField("name".to_string()))?
            .to_string();

        let domain = merged
            .get("domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 端口：完全按照 merged（local 优先）解析
        let port = merged
            .get("port")
            .and_then(Self::value_to_u16)
            .unwrap_or(8000) as u16;

        // 解析 debug 配置
        let debug = merged.get("debug").and_then(|v| {
            if let Value::Object(obj) = v {
                let mut map = std::collections::HashMap::new();
                for (key, val) in obj.iter() {
                    if let Some(url) = val.as_str() {
                        map.insert(key.clone(), url.to_string());
                    }
                }
                if map.is_empty() {
                    None
                } else {
                    Some(map)
                }
            } else {
                None
            }
        });

        let mut project = ProjectInfo::new(project_path.to_path_buf(), name);
        project.version = ZebrasVersion::V2;
        project.platform = platform;
        project.type_ = type_;
        project.domain = domain;
        project.port = port;
        project.framework = None; // V2 不指定框架
        project.debug = debug;

        Ok(project)
    }

    /// 解析 Zebras v3 项目配置 (TypeScript)
    pub fn parse_v3_config(project_path: &Path) -> Result<ProjectInfo, ParseError> {
        // 读取 zebras.config.ts
        let main_config_path = project_path.join("zebras.config.ts");
        if !main_config_path.exists() {
            return Err(ParseError::NotAZebrasProject);
        }

        let main_content = fs::read_to_string(&main_config_path)?;
        let main_config = ts_parser::parse_ts_config_simple(&main_content)
            .map_err(|e| ParseError::MissingField(e))?;

        // 读取 zebras.config.local.ts (可选)
        let local_config_path = project_path.join("zebras.config.local.ts");
        let (config, debug) = if local_config_path.exists() {
            let local_content = fs::read_to_string(&local_config_path)?;
            let local_config = ts_parser::parse_ts_config_simple(&local_content)
                .map_err(|e| ParseError::MissingField(e))?;
            let merged = ts_parser::merge_configs(&main_config, &local_config);

            // 解析 debug 配置（从 local 文件）
            let debug_map = ts_parser::parse_debug_config(&local_content);
            let debug = if debug_map.is_empty() {
                None
            } else {
                Some(debug_map)
            };

            (merged, debug)
        } else {
            (main_config, None)
        };

        // 提取字段
        let name = config
            .get("name")
            .ok_or_else(|| ParseError::MissingField("name".to_string()))?
            .clone();

        let platform = config.get("platform").map(|s| s.clone()).unwrap_or_else(|| "web".to_string());
        let type_ = config.get("type").map(|s| s.clone()).unwrap_or_else(|| "app".to_string());
        let domain = config.get("domain").map(|s| s.clone());
        let framework = config.get("framework").map(|s| s.clone());

        let port = config
            .get("port")
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(8000);

        let mut project = ProjectInfo::new(project_path.to_path_buf(), name);
        project.version = ZebrasVersion::V3;
        project.platform = platform;
        project.type_ = type_;
        project.domain = domain;
        project.port = port;
        project.framework = framework;
        project.debug = debug;

        Ok(project)
    }

    /// 自动检测并解析项目配置
    pub fn parse_project(project_path: &Path) -> Result<ProjectInfo, ParseError> {
        if let Some(version) = Self::detect_version_from_package_json(project_path) {
            println!(
                "[ConfigParser] Detected version {:?} via package.json for {}",
                version,
                project_path.display()
            );
            return match version {
                ZebrasVersion::V3 => Self::parse_v3_config(project_path),
                ZebrasVersion::V2 => Self::parse_v2_config(project_path),
            };
        }
        println!(
            "[ConfigParser] package.json unavailable or inconclusive for {}, fallback to file detection",
            project_path.display()
        );

        let has_v3 = project_path.join("zebras.config.ts").exists();
        let has_v2 = project_path.join("zebra.json").exists();

        match (has_v3, has_v2) {
            (false, false) => Err(ParseError::NotAZebrasProject),
            (true, false) => Self::parse_v3_config(project_path),
            (false, true) => Self::parse_v2_config(project_path),
            (true, true) => {
                // 两个配置文件都存在，尝试比较配置文件的修改时间
                let v3_path = project_path.join("zebras.config.ts");
                let v2_path = project_path.join("zebra.json");
                
                let v3_modified = fs::metadata(&v3_path).and_then(|m| m.modified()).ok();
                let v2_modified = fs::metadata(&v2_path).and_then(|m| m.modified()).ok();
                
                match (v3_modified, v2_modified) {
                    (Some(v3_time), Some(v2_time)) => {
                        if v3_time > v2_time {
                            println!(
                                "[ConfigParser] Both configs exist, zebras.config.ts is newer for {}",
                                project_path.display()
                            );
                            Self::parse_v3_config(project_path)
                        } else {
                            println!(
                                "[ConfigParser] Both configs exist, zebra.json is newer for {}",
                                project_path.display()
                            );
                            Self::parse_v2_config(project_path)
                        }
                    }
                    _ => {
                        // 无法比较时间，默认使用 V2（更保守的选择）
                        println!(
                            "[ConfigParser] Both configs exist, defaulting to V2 for {}",
                            project_path.display()
                        );
                        Self::parse_v2_config(project_path)
                    }
                }
            }
        }
    }

    /// 更新项目的本地配置文件中的端口
    pub fn update_port(project: &ProjectInfo, new_port: u16) -> Result<(), ParseError> {
        match project.version {
            ZebrasVersion::V2 => Self::update_v2_port(&project.path, new_port),
            ZebrasVersion::V3 => Self::update_v3_port(&project.path, new_port),
        }
    }

    /// 更新 Zebras v2 的端口配置
    fn update_v2_port(project_path: &Path, new_port: u16) -> Result<(), ParseError> {
        let local_path = project_path.join("zebra.local.json");

        let mut config: Value = if local_path.exists() {
            let content = fs::read_to_string(&local_path)?;
            serde_json::from_str(&content)?
        } else {
            Value::Object(serde_json::Map::new())
        };

        if let Some(obj) = config.as_object_mut() {
            obj.insert("port".to_string(), Value::Number(new_port.into()));
        }

        let json_string = serde_json::to_string_pretty(&config)?;
        fs::write(&local_path, json_string)?;

        Ok(())
    }

    /// 更新 Zebras v3 的端口配置
    fn update_v3_port(project_path: &Path, new_port: u16) -> Result<(), ParseError> {
        let local_path = project_path.join("zebras.config.local.ts");

        let content = if local_path.exists() {
            fs::read_to_string(&local_path)?
        } else {
            "export default {\n};\n".to_string()
        };

        let updated = ts_parser::update_port_in_ts(&content, new_port);
        fs::write(&local_path, updated)?;

        Ok(())
    }

    fn detect_version_from_package_json(project_path: &Path) -> Option<ZebrasVersion> {
        let package_path = project_path.join("package.json");
        let content = fs::read_to_string(&package_path).ok()?;
        
        // 只从 scripts.start 字段检测版本，不做全文搜索
        // 避免其他字段（如 "upgrade": "npm i -g zebras-cli"）干扰判断
        if let Some(start_script) = Self::extract_start_script(&content) {
            if let Some(version) = Self::determine_version_from_text(&start_script) {
                println!(
                    "[ConfigParser] Start script `{}` resolved to {:?} for {}",
                    start_script,
                    version,
                    project_path.display()
                );
                return Some(version);
            }
            println!(
                "[ConfigParser] Start script `{}` not recognized for {}",
                start_script,
                project_path.display()
            );
        } else {
            println!(
                "[ConfigParser] Could not extract start script from package.json for {}",
                project_path.display()
            );
        }
        
        // 不做全文搜索 fallback，让 parse_project 回退到文件检测
        None
    }

    fn extract_start_script(content: &str) -> Option<String> {
        if let Ok(package) = serde_json::from_str::<Value>(content) {
            if let Some(start) = package
                .get("scripts")
                .and_then(|scripts| scripts.get("start"))
                .and_then(|value| value.as_str())
            {
                return Some(start.to_string());
            }
        }

        // Fallback to regex when JSON 解析失败（可能存在尾随逗号等），并跳过被注释的行/块
        let regex = Regex::new(r#""start"\s*:\s*"([^"]+)""#).ok()?;
        let mut in_block_comment = false;

        for line in content.lines() {
            let mut slice = line;

            if in_block_comment {
                if let Some(end_idx) = slice.find("*/") {
                    in_block_comment = false;
                    slice = &slice[end_idx + 2..];
                } else {
                    continue;
                }
            }

            loop {
                if let Some(start_idx) = slice.find("/*") {
                    let (before, after_start) = slice.split_at(start_idx);
                    if let Some(captures) = regex.captures(before) {
                        return Some(captures.get(1)?.as_str().to_string());
                    }

                    if let Some(end_idx) = after_start[2..].find("*/") {
                        slice = &after_start[2 + end_idx + 2..];
                        continue;
                    } else {
                        in_block_comment = true;
                        break;
                    }
                }

                let uncommented = if let Some(comment_idx) = slice.find("//") {
                    let first_quote_idx = slice
                        .char_indices()
                        .find(|(_, c)| *c == '\'' || *c == '"')
                        .map(|(idx, _)| idx);

                    if first_quote_idx.map_or(true, |q_idx| comment_idx < q_idx) {
                        &slice[..comment_idx]
                    } else {
                        slice
                    }
                } else {
                    slice
                };

                if let Some(captures) = regex.captures(uncommented) {
                    return Some(captures.get(1)?.as_str().to_string());
                }

                break;
            }
        }

        None
    }

    /// 从文本中检测版本
    /// V3 使用 "zebras"（复数），V2 使用 "zebra"（单数）
    fn determine_version_from_text(text: &str) -> Option<ZebrasVersion> {
        let normalized = text.to_lowercase();
        
        // 先检查 V3（zebras，复数），因为 "zebras" 包含 "zebra" 作为子串
        // 必须先检查更长的字符串
        if normalized.contains("zebras") {
            return Some(ZebrasVersion::V3);
        }
        
        // 然后检查 V2（zebra，单数）
        if normalized.contains("zebra") {
            return Some(ZebrasVersion::V2);
        }
        
        None
    }

    fn merge_json(base: &Value, overlay: &Value) -> Value {
        match (base, overlay) {
            (Value::Object(base_obj), Value::Object(overlay_obj)) => {
                let mut merged = base_obj.clone();
                for (key, value) in overlay_obj {
                    let new_value = if let Some(existing) = merged.get(key) {
                        Self::merge_json(existing, value)
                    } else {
                        value.clone()
                    };
                    merged.insert(key.clone(), new_value);
                }
                Value::Object(merged)
            }
            (_, Value::Null) => base.clone(),
            (_, overlay_value) => overlay_value.clone(),
        }
    }

    fn value_to_u16(value: &Value) -> Option<u16> {
        if let Some(number) = value.as_u64() {
            return Some(number as u16);
        }
        if let Some(text) = value.as_str() {
            return text.parse::<u16>().ok();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_project_not_found() {
        let path = PathBuf::from("/nonexistent/path");
        let result = ConfigParser::parse_project(&path);
        assert!(result.is_err());
    }

    #[test]
    fn extract_start_script_ignores_line_comment() {
        let content = r#"
{
  "scripts": {
    // "start": "zebra dev"
    "build": "echo ok"
  }
}
"#;

        let result = ConfigParser::extract_start_script(content);
        assert!(result.is_none());
    }

    #[test]
    fn extract_start_script_skips_block_comment_and_reads_real_value() {
        let content = r#"
{
  /* "start": "zebra dev" */
  "scripts": {
    "start": "zebras dev" // trailing comment
  }
}
"#;

        let result = ConfigParser::extract_start_script(content);
        assert_eq!(result.as_deref(), Some("zebras dev"));
    }

    #[test]
    fn extract_start_script_ignores_multiline_block_comment() {
        let content = r#"
{
  /*
   * "start": "zebra dev"
   */
  "scripts": {
    "build": "echo ok"
  }
}
"#;

        let result = ConfigParser::extract_start_script(content);
        assert!(result.is_none());
    }

    #[test]
    fn extract_start_script_preserves_url_in_fallback() {
        let content = r#"
{
  "scripts": {
    "start": "vite --host http://localhost:3000",
  }
}
"#;

        // Invalid JSON (trailing comma) forces fallback regex
        let result = ConfigParser::extract_start_script(content);
        assert_eq!(
            result.as_deref(),
            Some("vite --host http://localhost:3000")
        );
    }
}
