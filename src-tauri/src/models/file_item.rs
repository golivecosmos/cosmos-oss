use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub child_count: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct FileMetadata {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    pub modified_at: String,
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub dimensions: Option<(u32, u32)>,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub frame_number: Option<u32>,
}
