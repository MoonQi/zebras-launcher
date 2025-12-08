use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// 使用正则表达式解析简单的 TypeScript 配置文件
/// 这是一个备用方案，适用于标准格式的配置文件
pub fn parse_ts_config_simple(content: &str) -> Result<HashMap<String, String>, String> {
    let mut config = HashMap::new();

    // 提取 port
    if let Some(cap) = Regex::new(r#"port:\s*['"]?(\d+)['"]?"#)
        .unwrap()
        .captures(content)
    {
        config.insert("port".to_string(), cap[1].to_string());
    }

    // 提取 name
    if let Some(cap) = Regex::new(r#"name:\s*['"]([^'"]+)['"]"#)
        .unwrap()
        .captures(content)
    {
        config.insert("name".to_string(), cap[1].to_string());
    }

    // 提取 domain
    if let Some(cap) = Regex::new(r#"domain:\s*['"]([^'"]+)['"]"#)
        .unwrap()
        .captures(content)
    {
        config.insert("domain".to_string(), cap[1].to_string());
    }

    // 提取 type
    if let Some(cap) = Regex::new(r#"type:\s*['"]([^'"]+)['"]"#)
        .unwrap()
        .captures(content)
    {
        config.insert("type".to_string(), cap[1].to_string());
    }

    // 提取 platform
    if let Some(cap) = Regex::new(r#"platform:\s*['"]([^'"]+)['"]"#)
        .unwrap()
        .captures(content)
    {
        config.insert("platform".to_string(), cap[1].to_string());
    }

    // 提取 framework
    if let Some(cap) = Regex::new(r#"framework:\s*['"]([^'"]+)['"]"#)
        .unwrap()
        .captures(content)
    {
        config.insert("framework".to_string(), cap[1].to_string());
    }

    Ok(config)
}

/// 解析 TypeScript 配置文件中的 debug 对象
/// 返回 HashMap<项目名, URL>
pub fn parse_debug_config(content: &str) -> HashMap<String, String> {
    let mut debug_map = HashMap::new();

    // 匹配 debug: { ... } 块
    let debug_block_regex = Regex::new(r"debug:\s*\{([^}]*)\}").unwrap();

    if let Some(cap) = debug_block_regex.captures(content) {
        let debug_content = &cap[1];

        // 匹配每一行的 key: 'value' 或 key: "value"
        let entry_regex = Regex::new(r#"(\w+):\s*['"]([^'"]+)['"]"#).unwrap();

        for cap in entry_regex.captures_iter(debug_content) {
            let key = cap[1].to_string();
            let value = cap[2].to_string();
            debug_map.insert(key, value);
        }
    }

    debug_map
}

/// 更新 TypeScript 配置文件中的端口
pub fn update_port_in_ts(content: &str, new_port: u16) -> String {
    let port_regex = Regex::new(r#"port:\s*['"]?\d+['"]?"#).unwrap();

    if port_regex.is_match(content) {
        // 替换现有的 port 字段
        port_regex
            .replace(content, &format!("port: '{}'", new_port))
            .to_string()
    } else {
        // 插入新的 port 字段
        content.replace(
            "export default {",
            &format!("export default {{\n    port: '{}',", new_port),
        )
    }
}

/// 更新 TypeScript 配置文件中的 debug 对象
pub fn update_debug_in_ts(content: &str, debug_map: &HashMap<String, String>) -> String {
    let debug_block_regex = Regex::new(r"debug:\s*\{[^}]*\}").unwrap();

    // 构建新的 debug 对象字符串
    let debug_entries: Vec<String> = debug_map
        .iter()
        .map(|(k, v)| format!("        {}: '{}'", k, v))
        .collect();

    let new_debug_block = if debug_entries.is_empty() {
        "".to_string()
    } else {
        format!("debug: {{\n{},\n    }}", debug_entries.join(",\n"))
    };

    if debug_block_regex.is_match(content) {
        // 替换现有的 debug 块
        if new_debug_block.is_empty() {
            // 删除 debug 块（包括可能的逗号）
            let result = debug_block_regex.replace(content, "").to_string();
            // 清理可能的双逗号
            result.replace(",,", ",")
        } else {
            debug_block_regex
                .replace(content, &new_debug_block)
                .to_string()
        }
    } else {
        // 插入新的 debug 块（如果有内容）
        if new_debug_block.is_empty() {
            content.to_string()
        } else {
            // 在 export default { 后插入
            content.replace(
                "export default {",
                &format!("export default {{\n    {},", new_debug_block),
            )
        }
    }
}

/// 合并两个配置 HashMap（local 配置覆盖 main 配置）
pub fn merge_configs(
    main: &HashMap<String, String>,
    local: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = main.clone();
    for (key, value) in local.iter() {
        result.insert(key.clone(), value.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ts_config_simple() {
        let content = r#"
export default {
    type: 'app',
    domain: 'yilu',
    name: 'yilu_filing',
    port: '8000',
    platform: 'web',
    framework: 'react',
};
        "#;

        let config = parse_ts_config_simple(content).unwrap();
        assert_eq!(config.get("name"), Some(&"yilu_filing".to_string()));
        assert_eq!(config.get("port"), Some(&"8000".to_string()));
        assert_eq!(config.get("platform"), Some(&"web".to_string()));
    }

    #[test]
    fn test_update_port_in_ts() {
        let content = r#"export default {
    port: '8000',
    name: 'test',
};"#;

        let updated = update_port_in_ts(content, 8001);
        assert!(updated.contains("port: '8001'"));
    }
}
