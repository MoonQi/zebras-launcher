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
pub struct GitPullResult {
    pub success: bool,
    pub message: String,
    pub status: GitStatus,
}
