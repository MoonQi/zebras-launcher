use crate::models::{ProjectInfo, ZebrasVersion};
use crate::utils::ts_parser;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[tauri::command]
pub async fn update_debug_config(
    project_path: String,
    project_version: String,
    debug_map: HashMap<String, String>,
) -> Result<(), String> {
    let path = PathBuf::from(&project_path);

    match project_version.as_str() {
        "v3" => update_v3_debug(&path, &debug_map),
        "v2" => update_v2_debug(&path, &debug_map),
        _ => Err("不支持的项目版本".to_string()),
    }
}

fn update_v3_debug(project_path: &Path, debug_map: &HashMap<String, String>) -> Result<(), String> {
    let local_config_path = project_path.join("zebras.config.local.ts");

    let content = if local_config_path.exists() {
        fs::read_to_string(&local_config_path).map_err(|e| e.to_string())?
    } else {
        // 如果 local 配置不存在，创建一个基础模板
        "export default {\n};\n".to_string()
    };

    let updated = ts_parser::update_debug_in_ts(&content, debug_map);

    fs::write(&local_config_path, updated).map_err(|e| e.to_string())?;

    Ok(())
}

fn update_v2_debug(project_path: &Path, debug_map: &HashMap<String, String>) -> Result<(), String> {
    let local_config_path = project_path.join("zebra.local.json");

    let mut config: Value = if local_config_path.exists() {
        let json_str = fs::read_to_string(&local_config_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json_str).map_err(|e| e.to_string())?
    } else {
        serde_json::json!({})
    };

    if debug_map.is_empty() {
        // 删除 debug 字段
        if let Value::Object(ref mut obj) = config {
            obj.remove("debug");
        }
    } else {
        // 更新 debug 字段
        let debug_obj: serde_json::Map<String, Value> = debug_map
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();

        config["debug"] = Value::Object(debug_obj);
    }

    let json_str = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&local_config_path, json_str).map_err(|e| e.to_string())?;

    Ok(())
}
