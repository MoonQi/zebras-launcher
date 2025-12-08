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

        let mut in_block_comment = false;
        for line in debug_content.lines() {
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
                    for cap in entry_regex.captures_iter(before) {
                        debug_map.insert(cap[1].to_string(), cap[2].to_string());
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

                for cap in entry_regex.captures_iter(uncommented) {
                    debug_map.insert(cap[1].to_string(), cap[2].to_string());
                }

                break;
            }
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
    // 捕获 debug 块（包含可能存在的结尾逗号）
    let debug_block_regex = Regex::new(r"(?s)debug:\s*\{.*?\}\s*,?").unwrap();

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

    if let Some(mat) = debug_block_regex.find(content) {
        // 替换现有的 debug 块
        if new_debug_block.is_empty() {
            // 删除 debug 块（包括可能的逗号）
            let mut result = String::new();
            result.push_str(&content[..mat.start()]);
            result.push_str(&content[mat.end()..]);

            // 清理可能遗留的逗号和空行：
            // 1) 连续逗号 -> 单逗号
            let no_double_commas = Regex::new(r",\s*,").unwrap().replace_all(&result, ",").to_string();
            // 2) 左大括号后紧跟逗号 -> 去掉逗号
            let no_leading_comma = Regex::new(r"\{\s*,").unwrap().replace_all(&no_double_commas, "{\n").to_string();
            // 3) 结尾逗号紧贴右大括号 -> 去掉逗号
            let no_trailing_comma = Regex::new(r",\s*}").unwrap().replace_all(&no_leading_comma, "\n}").to_string();
            // 4) 压缩多余空行
            Regex::new(r"\n\s*\n\s*\n+").unwrap().replace_all(&no_trailing_comma, "\n\n").to_string()
        } else {
            // 替换成新的 debug 块，并根据后续内容决定是否需要逗号
            let before = &content[..mat.start()];
            let after = &content[mat.end()..];

            let needs_comma_after = after
                .chars()
                .skip_while(|c| c.is_whitespace() || *c == '\n' || *c == '\r')
                .next()
                .map_or(false, |c| c != '}');

            let mut block = new_debug_block.clone();
            if needs_comma_after {
                block.push(',');
            }

            let mut updated = String::new();
            updated.push_str(before);
            updated.push_str(&block);
            updated.push_str(after);
            updated
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

    #[test]
    fn parse_debug_config_ignores_line_comment() {
        let content = r#"
export default {
    debug: {
        // yilu_office: 'http://localhost:7010',
        yilu_filing: 'http://localhost:8633',
    }
};
"#;

        let debug = parse_debug_config(content);
        assert_eq!(debug.get("yilu_office"), None);
        assert_eq!(debug.get("yilu_filing"), Some(&"http://localhost:8633".to_string()));
    }

    #[test]
    fn parse_debug_config_ignores_block_comment() {
        let content = r#"
export default {
    debug: {
        /* yilu_office: 'http://localhost:7010', */
        yilu_filing: 'http://localhost:8633',
    }
};
"#;

        let debug = parse_debug_config(content);
        assert_eq!(debug.get("yilu_office"), None);
        assert_eq!(debug.get("yilu_filing"), Some(&"http://localhost:8633".to_string()));
    }

    #[test]
    fn parse_debug_config_handles_trailing_comment_after_value() {
        let content = r#"
export default {
    debug: {
        yilu_filing: 'http://localhost:8633', // trailing comment
    }
};
"#;

        let debug = parse_debug_config(content);
        assert_eq!(debug.get("yilu_filing"), Some(&"http://localhost:8633".to_string()));
    }

    #[test]
    fn update_debug_in_ts_removes_block_cleanly_when_only_debug_exists() {
        let content = r#"
export default {
    debug: {
        yilu_filing: 'http://localhost:8633',
    },
};
"#;

        let updated = update_debug_in_ts(content, &HashMap::new());
        assert!(!updated.contains("debug"));
        assert!(!updated.contains(",\n"));
        assert!(updated.contains("export default {"));
    }

    #[test]
    fn update_debug_in_ts_removes_block_without_breaking_other_fields() {
        let content = r#"
export default {
    debug: {
        yilu_filing: 'http://localhost:8633',
    },
    port: '8000',
};
"#;

        let updated = update_debug_in_ts(content, &HashMap::new());
        assert!(!updated.contains("debug"));
        assert!(updated.contains("port: '8000'"));
        assert!(updated.contains("export default {"));
    }

    #[test]
    fn update_debug_in_ts_removes_block_when_debug_last_field() {
        let content = r#"
export default {
    port: '8000',
    name: 'demo',
    debug: {
        yilu_filing: 'http://localhost:8633',
    },
};
"#;

        let updated = update_debug_in_ts(content, &HashMap::new());
        assert!(!updated.contains("debug"));
        assert!(updated.contains("port: '8000'"));
        assert!(updated.contains("name: 'demo'"));
        assert!(!updated.contains(",\n}")); // 不应留下末尾逗号
    }

    #[test]
    fn update_debug_in_ts_inserts_comma_when_followed_by_other_fields() {
        let mut map = HashMap::new();
        map.insert("yilu_filing".to_string(), "http://localhost:8633".to_string());
        map.insert("yilu_office".to_string(), "http://localhost:7010".to_string());

        let content = r#"
export default {
    port: '7000',
};
"#;

        let updated = update_debug_in_ts(content, &map);
        assert!(updated.contains("debug: {"));
        assert!(updated.contains("yilu_office"));
        assert!(updated.contains("port: '7000'"));
        assert!(updated.contains("},\n    port")); // debug 后应有逗号再到 port
    }
}
