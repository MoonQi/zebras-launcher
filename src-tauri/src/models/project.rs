use super::{ProjectSourceType, ProvisionStatus, RepoRole};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub path: PathBuf,
    pub version: ZebrasVersion,
    #[serde(default = "default_project_source_type")]
    pub source_type: ProjectSourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_role: Option<RepoRole>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provision_status: Option<ProvisionStatus>,
    pub platform: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub domain: Option<String>,
    pub port: u16,
    pub framework: Option<String>,
    pub is_valid: bool,
    pub last_scanned: DateTime<Utc>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<HashMap<String, String>>, // 调试依赖配置
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub enabled: Option<bool>, // 是否在"全部启动"时启动，默认 true
    #[serde(default = "default_runnable")]
    pub runnable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZebrasVersion {
    V2,
    V3,
    Managed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortChange {
    pub project_name: String,
    pub old_port: u16,
    pub new_port: u16,
}

impl ProjectInfo {
    pub fn new(path: PathBuf, name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            version: ZebrasVersion::V3,
            source_type: ProjectSourceType::Zebras,
            repo_role: None,
            provision_status: None,
            platform: "web".to_string(),
            type_: "app".to_string(),
            name,
            domain: None,
            port: 8000,
            framework: None,
            is_valid: true,
            last_scanned: Utc::now(),
            error: None,
            debug: None,
            enabled: Some(true), // 默认启用
            runnable: true,
        }
    }
}

fn default_project_source_type() -> ProjectSourceType {
    ProjectSourceType::Zebras
}

fn default_runnable() -> bool {
    true
}
