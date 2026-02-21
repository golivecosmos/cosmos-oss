/// Supported image file extensions
/// 
/// This constant is used throughout the application to determine which file types
/// are supported for indexing, searching, and display. When adding new image formats,
/// update this constant and the corresponding frontend constant in src/constants.ts
pub const SUPPORTED_IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"
];

/// Supported video file extensions
pub const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "avi", "webm", "mkv", "flv", "wmv", "m4v"
];

/// Check if a file extension is a supported video type
pub fn is_supported_video_extension(ext: &str) -> bool {
    SUPPORTED_VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}

/// Check if a file extension is a supported media type
pub fn is_supported_media_extension(ext: &str) -> bool {
    let ext_lower = ext.to_lowercase();
    SUPPORTED_IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) || 
    SUPPORTED_VIDEO_EXTENSIONS.contains(&ext_lower.as_str())
}

// ============================================================================
// MODEL DOWNLOAD SETTINGS
// ============================================================================
// Cosmos OSS no longer relies on private endpoints. All model locations are
// configurable via environment variables so operators can mirror artifacts or
// point to official Hugging Face releases without rebuilding the app.

const DEFAULT_MODEL_BASE_URL: &str = "https://huggingface.co";
const DEFAULT_MODEL_NAMESPACE: &str = "nomic-ai";
const DEFAULT_TEXT_MODEL_SLUG: &str = "nomic-embed-text-v1.5/resolve/main";
const DEFAULT_VISION_MODEL_SLUG: &str = "nomic-embed-vision-v1.5/resolve/main";

/// Base URL for the model registry (defaults to Hugging Face).
pub fn model_registry_base_url() -> String {
    std::env::var("COSMOS_MODEL_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_MODEL_BASE_URL.to_string())
}

/// Optional namespace/organization segment (defaults to `nomic-ai`).
pub fn model_namespace() -> String {
    std::env::var("COSMOS_MODEL_NAMESPACE")
        .unwrap_or_else(|_| DEFAULT_MODEL_NAMESPACE.to_string())
}

/// Text embedding model slug (can include extra path segments like `resolve/main`).
pub fn text_model_slug() -> String {
    std::env::var("COSMOS_TEXT_MODEL_SLUG")
        .unwrap_or_else(|_| DEFAULT_TEXT_MODEL_SLUG.to_string())
}

/// Vision embedding model slug (can include extra path segments like `resolve/main`).
pub fn vision_model_slug() -> String {
    std::env::var("COSMOS_VISION_MODEL_SLUG")
        .unwrap_or_else(|_| DEFAULT_VISION_MODEL_SLUG.to_string())
}

/// Database filename for vector search database
pub const DATABASE_FILENAME: &str = ".cosmos.db";
