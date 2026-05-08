use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSourceType {
    FolderScan,
    ManagedProject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSourceType {
    Zebras,
    ManagedProject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepoRole {
    FrontendApp,
    BackendService,
    FrontendPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionStatus {
    Pending,
    Provisioning,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedRepo {
    pub id: String,
    pub display_name: String,
    pub role: RepoRole,
    pub git_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub mount_path: String,
    pub status: ProvisionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedFrontendLink {
    pub provider_repo_id: String,
    pub consumer_repo_id: String,
    pub status: ProvisionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedProjectManifest {
    pub version: u32,
    pub project_name: String,
    pub root_path: PathBuf,
    pub status: ProvisionStatus,
    pub repos: Vec<ManagedRepo>,
    pub frontend_links: Vec<ManagedFrontendLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedRepoInput {
    pub id: String,
    pub display_name: String,
    pub role: RepoRole,
    pub git_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedFrontendLinkInput {
    pub provider_repo_id: String,
    pub consumer_repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectInstanceInput {
    pub project_name: String,
    pub root_path: String,
    pub repos: Vec<ManagedRepoInput>,
    #[serde(default)]
    pub frontend_links: Vec<ManagedFrontendLinkInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}
