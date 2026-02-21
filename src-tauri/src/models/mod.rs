pub mod nomic;
pub mod embedding;
pub mod file_item;
pub mod whisper;

#[cfg(test)]
pub mod tests;

use anyhow::Result;
use image::DynamicImage;

/// Trait for embedding models (CLIP, Nomic, etc.)
pub trait EmbeddingModel: Send + Sync {
    /// Encode text into an embedding vector
    fn encode_text(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Encode a single image into an embedding vector
    fn encode_image(&self, img: &DynamicImage) -> Result<Vec<f32>>;
} 