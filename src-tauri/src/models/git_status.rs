use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub has_remote: bool,
    pub uncommitted_count: u32,
    pub ahead_count: u32,
    pub behind_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub upstream: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPullResult {
    pub success: bool,
    pub message: String,
    pub status: GitStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSwitchResult {
    pub message: String,
    pub status: GitStatus,
}
