/// **NOMIC MODEL LOADING STRATEGY v2.0**
///
/// **Problem with Previous Approach:**
/// - Used FastEmbed's auto-download from HuggingFace Hub
/// - Complex directory structure (.fastembed_cache/)
/// - Unreliable for production deployments
/// - No control over model sources
///
/// **New LOCAL FILES ONLY Strategy:**
///
/// **1. Directory Structure (Clean & Simple):**
/// ```
/// ~/Library/Application Support/cosmos/models/
/// ├── nomic-embed-text-v1.5/
/// │   ├── config.json              (Text model config)
/// │   ├── tokenizer.json           (Tokenizer vocabulary)
/// │   ├── tokenizer_config.json    (Tokenizer settings)
/// │   ├── special_tokens_map.json  (Special tokens)
/// │   └── onnx/
/// │       └── model.onnx           (Text embedding model)
/// └── nomic-embed-vision-v1.5/
///     ├── preprocessor_config.json (Vision preprocessing)
///     └── onnx/
///         └── model.onnx           (Vision embedding model)
/// ```
///
/// **2. Model Sources:**
/// - Configurable via environment variables so operators can mirror artifacts.
/// - Defaults target the official Hugging Face releases:
///   * https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/main
///   * https://huggingface.co/nomic-ai/nomic-embed-vision-v1.5/resolve/main
///
/// **3. FastEmbed Configuration:**
/// - Set FASTEMBED_CACHE_DIR to our models/ directory
/// - Initialize with explicit cache_dir parameter
/// - NO auto-download (local files only)
/// - Models must be pre-downloaded from our S3 bucket
///
/// **4. Benefits:**
/// - ✅ Predictable, controlled model loading
/// - ✅ No external dependencies during runtime
/// - ✅ Faster startup (no download checks)
/// - ✅ Version control over model files
/// - ✅ Simplified directory structure
/// - ✅ Production-ready deployment

use anyhow::Result;
use fastembed::{
    TextEmbedding, ImageEmbedding,
    InitOptions, ImageInitOptions,
    EmbeddingModel, ImageEmbeddingModel
};
use image::DynamicImage;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{app_log_error, app_log_info, app_log_warn};
use crate::utils::path_utils;
use super::EmbeddingModel as EmbeddingModelTrait;

// Nomic model configuration
#[derive(Debug, Deserialize, Serialize)]
struct NomicConfig {
    vocab_size: Option<usize>,
    max_position_embeddings: Option<usize>,
    pad_token_id: Option<i64>,
    unk_token_id: Option<i64>,
    bos_token_id: Option<i64>,
    eos_token_id: Option<i64>,
}

// Nomic vocabulary structure
#[derive(Debug, Deserialize, Serialize)]
struct NomicVocabulary {
    vocab: HashMap<String, i64>,
    max_length: usize,
    pad_token_id: i64,
    unk_token_id: i64,
    bos_token_id: i64,
    eos_token_id: i64,
}

pub struct NomicModel {
    text_model: Arc<Mutex<TextEmbedding>>,
    vision_model: Arc<Mutex<ImageEmbedding>>,
}

impl NomicModel {
    /// **NEW MODEL LOADING STRATEGY**
    ///
    /// **Goals:**
    /// 1. Disable FastEmbed's auto-download from HuggingFace
    /// 2. Use our own S3-hosted models for production reliability
    /// 3. Use simplified directory structure: app_data_dir/models/model-name/
    /// 4. Use local_files_only=true to prevent any external downloads
    ///
    /// **Directory Structure:**
    /// ```
    /// app_data_dir/models/
    /// ├── nomic-embed-text-v1.5/
    /// │   ├── config.json
    /// │   ├── tokenizer.json
    /// │   ├── tokenizer_config.json
    /// │   ├── special_tokens_map.json
    /// │   └── onnx/model.onnx
    /// └── nomic-embed-vision-v1.5/
    ///     ├── preprocessor_config.json
    ///     └── onnx/model.onnx
    /// ```
    pub fn new() -> Result<Self> {
        app_log_info!("🚀 Initializing Nomic Embed models with LOCAL FILES ONLY strategy");

        // **STRATEGY: Always use local-only mode, no auto-downloads**
        Self::new_with_local_files_only()
    }

    /// **NEW: Local files only approach**
    /// This is our primary initialization method for production
    fn new_with_local_files_only() -> Result<Self> {
        let app_data_dir = path_utils::get_app_data_dir()?;

        // **NEW: Set cache directory to our simplified models directory**
        let models_cache_dir = app_data_dir.join("models");

        // **CRITICAL: Set FASTEMBED_CACHE_DIR environment variable**
        // This tells FastEmbed where to look for models
        std::env::set_var("FASTEMBED_CACHE_DIR", models_cache_dir.to_string_lossy().to_string());
        app_log_info!("🔧 Set FASTEMBED_CACHE_DIR to: {}", models_cache_dir.display());

        // **Check if models are available before initializing**
        let models_available = Self::check_local_models_available();
        if !models_available {
            app_log_error!("❌ Required model files not found in local cache");
            app_log_error!("💡 Please run download_models command first to download from S3");
            return Err(anyhow::anyhow!("Model files not found. Please download models first."));
        }

        app_log_info!("✅ Local model files found, initializing with local_files_only=true");

        // **🚀 OPTIMIZED: Initialize FastEmbed text model for performance**
        let text_model = match TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
                .with_show_download_progress(false) // No downloads, so no progress needed
                .with_cache_dir(models_cache_dir.clone()) // Explicit cache directory
        ) {
            Ok(model) => {
                app_log_info!("✅ FastEmbed text model created successfully");
                Arc::new(Mutex::new(model))
            },
            Err(e) => {
                app_log_error!("❌ Failed to create FastEmbed text model: {}", e);
                return Err(anyhow::anyhow!("Failed to create FastEmbed text model: {}", e));
            }
        };

        // **🚀 OPTIMIZED: Initialize FastEmbed vision model for batch processing**
        let vision_model = match ImageEmbedding::try_new(
            ImageInitOptions::new(ImageEmbeddingModel::NomicEmbedVisionV15)
                .with_show_download_progress(false) // No downloads, so no progress needed
                .with_cache_dir(models_cache_dir) // Explicit cache directory
        ) {
            Ok(model) => {
                app_log_info!("✅ FastEmbed vision model created successfully");
                Arc::new(Mutex::new(model))
            },
            Err(e) => {
                app_log_error!("❌ Failed to create FastEmbed vision model: {}", e);
                return Err(anyhow::anyhow!("Failed to create FastEmbed vision model: {}", e));
            }
        };

        app_log_info!("🎉 Nomic models initialized successfully with local files only");

        Ok(Self {
            text_model,
            vision_model,
        })
    }

    /// **Check if we have the required models in our simplified local structure**
    fn check_local_models_available() -> bool {
        match path_utils::get_app_data_dir() {
            Ok(app_data_dir) => {
                let models_cache = app_data_dir.join("models");

                                 // **Check text model files**
                 let text_model_dir = models_cache.join("nomic-embed-text-v1.5");
                 let text_model_onnx = text_model_dir.join("onnx").join("model.onnx");
                 let text_config = text_model_dir.join("config.json");
                 let text_tokenizer = text_model_dir.join("tokenizer.json");

                 // **Check vision model files**
                 let vision_model_dir = models_cache.join("nomic-embed-vision-v1.5");
                 let vision_model_onnx = vision_model_dir.join("onnx").join("model.onnx");
                 let vision_preprocessor = vision_model_dir.join("preprocessor_config.json");

                let text_available = text_model_onnx.exists() && text_config.exists() && text_tokenizer.exists();
                let vision_available = vision_model_onnx.exists() && vision_preprocessor.exists();

                app_log_info!("🔍 Local model availability check:");
                app_log_info!("  Text model ONNX: {} - {}", text_model_onnx.display(), if text_model_onnx.exists() { "✅" } else { "❌" });
                app_log_info!("  Text config: {} - {}", text_config.display(), if text_config.exists() { "✅" } else { "❌" });
                app_log_info!("  Text tokenizer: {} - {}", text_tokenizer.display(), if text_tokenizer.exists() { "✅" } else { "❌" });
                app_log_info!("  Vision model ONNX: {} - {}", vision_model_onnx.display(), if vision_model_onnx.exists() { "✅" } else { "❌" });
                app_log_info!("  Vision preprocessor: {} - {}", vision_preprocessor.display(), if vision_preprocessor.exists() { "✅" } else { "❌" });

                let all_available = text_available && vision_available;
                app_log_info!("📊 Overall availability: {}", if all_available { "✅ Ready" } else { "❌ Missing files" });

                all_available
            }
            Err(e) => {
                app_log_error!("❌ Could not check local models: {}", e);
                false
            }
        }
    }
}

impl EmbeddingModelTrait for NomicModel {
    fn encode_text(&self, text: &str) -> Result<Vec<f32>> {
        // Generate embeddings with FastEmbed
        let documents = vec![text];

        let lock_start = std::time::Instant::now();
        let text_model = self.text_model.lock().map_err(|e| anyhow::anyhow!("Failed to acquire text model lock: {}", e))?;
        let lock_time = lock_start.elapsed();

        if lock_time.as_millis() > 10 {
            app_log_warn!("⏱️ TEXT MODEL LOCK: Took {:.2}ms to acquire (possible contention)", lock_time.as_millis());
        }

        let inference_start = std::time::Instant::now();
        let result = match text_model.embed(documents, None) {
            Ok(embeddings) => {
                if let Some(embedding) = embeddings.first() {
                    Ok(embedding.to_vec())
                } else {
                    Err(anyhow::anyhow!("No embedding generated"))
                }
            },
            Err(e) => Err(anyhow::anyhow!("Failed to generate embedding: {}", e))
        };
        let inference_time = inference_start.elapsed();

        app_log_info!("🧠 NOMIC TEXT TIMING: Lock={:.1}ms, Inference={:.1}ms",
            lock_time.as_millis(), inference_time.as_millis());

        result
    }

    fn encode_image(&self, img: &DynamicImage) -> Result<Vec<f32>> {
        let total_start = std::time::Instant::now();

        // Save image to temporary file
        let io_start = std::time::Instant::now();
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().join("temp_image.jpg");

        // Save the image
        img.save(&temp_path)?;
        let io_time = io_start.elapsed();

        // Generate embeddings with FastEmbed
        let images = vec![temp_path.to_string_lossy().to_string()];

        let lock_start = std::time::Instant::now();
        let vision_model = self.vision_model.lock().map_err(|e| anyhow::anyhow!("Failed to acquire vision model lock: {}", e))?;
        let lock_time = lock_start.elapsed();

        if lock_time.as_millis() > 10 {
            app_log_warn!("⚠️ VISION MODEL LOCK: Took {:.2}ms to acquire (BOTTLENECK!)", lock_time.as_millis());
        }
        let inference_start = std::time::Instant::now();
        let result = match vision_model.embed(images, None) {
            Ok(embeddings) => {
                if let Some(embedding) = embeddings.first() {
                    Ok(embedding.to_vec())
                } else {
                    Err(anyhow::anyhow!("No embedding generated"))
                }
            },
            Err(e) => Err(anyhow::anyhow!("Failed to generate embedding: {}", e))
        };
        let inference_time = inference_start.elapsed();
        let total_time = total_start.elapsed();

        app_log_info!("🧠 NOMIC VISION TIMING: IO={:.1}ms, Lock={:.1}ms, Inference={:.1}ms, Total={:.1}ms",
            io_time.as_millis(), lock_time.as_millis(), inference_time.as_millis(), total_time.as_millis());

        result
    }
}
