use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use serde_json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector(pub Vec<f32>);

impl From<Vec<u8>> for EmbeddingVector {
    fn from(bytes: Vec<u8>) -> Self {
        Self(
            bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                .collect()
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageVectorData {
    pub id: String,
    pub file_path: String,
    pub metadata: String,
    pub score: f32,
    pub embedding: EmbeddingVector,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_indexed_at: Option<DateTime<Utc>>,
    pub mime_type: Option<String>,
    pub parent_file_path: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ImageVectorDataResponse {
    /// Unique identifier for the image
    pub id: String,
    
    /// Full path to the file
    pub file_path: String,
    
    /// Metadata as a JSON string containing image attributes:
    /// dimensions: { width, height }, color_type, aspect_ratio, is_directory, fs_size, etc.
    pub metadata: String,
    
    /// Similarity score (higher is better match)
    pub score: f32,
    
    /// Current status of the image (e.g., "indexed", "pending", etc.)
    pub status: String,
    
    /// ISO 8601 timestamp when the image was first indexed
    pub created_at: String,
    
    /// ISO 8601 timestamp when the image was last updated
    pub updated_at: String,
    
    /// ISO 8601 timestamp when the image was last indexed (may be null)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_indexed_at: Option<String>,
    
    /// MIME type of the file (e.g., "image/png", "image/jpeg")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    
    /// Directory path containing the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_file_path: Option<String>,
    
    /// Comma-separated list of tags assigned to the image
    pub tags: String,
    
    /// Add video frame specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_formatted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_duration: Option<f64>,
    
    /// Drive information for files on external drives
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_custom_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_physical_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drive_status: Option<String>,
}

impl From<ImageVectorData> for ImageVectorDataResponse {
    fn from(data: ImageVectorData) -> Self {
        Self {
            id: data.id,
            file_path: data.file_path,
            metadata: data.metadata,
            score: data.score,
            status: data.status,
            created_at: data.created_at.to_rfc3339(),
            updated_at: data.updated_at.to_rfc3339(),
            last_indexed_at: data.last_indexed_at.map(|t| t.to_rfc3339()),
            mime_type: data.mime_type,
            parent_file_path: data.parent_file_path,
            tags: data.tags.unwrap_or_default(),
            timestamp: None,
            timestamp_formatted: None,
            frame_number: None,
            video_duration: None,
            drive_uuid: None,
            drive_name: None,
            drive_custom_name: None,
            drive_physical_location: None,
            drive_status: None,
        }
    }
}

impl ImageVectorDataResponse {
    /// Check if this response represents a video frame
    pub fn is_video_frame(&self) -> bool {
        // Must have timestamp and video_duration data to be a frame
        if self.timestamp.is_some() && self.video_duration.is_some() {
            return true;
        }
        
        // Check mime type
        if let Some(mime) = &self.mime_type {
            if mime == "video/frame" {
                return true;
            }
        }
        
        // Check ID format (frames have specific ID format)
        if self.id.contains(":frame:") {
            return true;
        }
        
        // Parse metadata to check for source_type
        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&self.metadata) {
            if let Some(source_type) = metadata.get("source_type").and_then(|v| v.as_str()) {
                if source_type == "video_frame" {
                    return true;
                }
            }
        }
        
        false
    }
}

/// Structure to store metadata about video frames
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFrameMetadata {
    pub video_path: String,        // Path to the source video file
    pub timestamp: f64,            // Timestamp in seconds where the frame occurs
    pub frame_number: usize,       // Frame number in the video
    pub video_duration: f64,       // Total duration of the video in seconds
    pub video_width: u32,          // Width of the video
    pub video_height: u32,         // Height of the video
}