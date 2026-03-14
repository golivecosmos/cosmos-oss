use crate::constants::{
    model_namespace, model_registry_base_url, text_model_slug, vision_model_slug,
};
use crate::utils::path_utils;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub file_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f32,
    pub status: DownloadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WhisperStatus {
    NotAvailable,
    Downloading,
    Ready,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ModelFile {
    pub name: String,
    pub url: String,
    pub destination_path: PathBuf,
}

pub struct DownloadService {
    client: reqwest::Client,
}

// Global state to track whisper download status
lazy_static::lazy_static! {
    static ref WHISPER_DOWNLOAD_STATE: Arc<Mutex<WhisperStatus>> = Arc::new(Mutex::new(WhisperStatus::NotAvailable));
}

impl DownloadService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Get list of required Nomic model files to download
    ///
    /// **Strategy Documentation:**
    /// 1. Store models in simplified structure: app_data_dir/models/models--org--name/
    /// 2. This matches FastEmbed's expected cache structure
    /// 3. Allow alternate registries (defaults to Hugging Face) via env vars
    /// 4. Set FASTEMBED_CACHE_DIR to app_data_dir/models for FastEmbed to find them
    pub fn get_required_models() -> Result<Vec<ModelFile>> {
        let app_data_dir = path_utils::get_app_data_dir()?;

        // **NEW: Simplified directory structure**
        // No more .fastembed_cache - just use models/ directory directly
        let models_cache = app_data_dir.join("models");

        // **SIMPLIFIED: Use clean, readable naming convention**
        let text_model_dir = models_cache.join("nomic-embed-text-v1.5");
        let text_onnx_dir = text_model_dir.join("onnx");

        let vision_model_dir = models_cache.join("nomic-embed-vision-v1.5");
        let vision_onnx_dir = vision_model_dir.join("onnx");

        let whisper_model_dir = models_cache.join("whisper-base");

        // Only create directories if we're not in a migration scenario
        if !path_utils::is_migration_needed() {
            fs::create_dir_all(&text_onnx_dir)?;
            fs::create_dir_all(&vision_onnx_dir)?;
            fs::create_dir_all(&whisper_model_dir)?;
        }

        app_log_info!("📁 Model directories:");
        app_log_info!("  Text model: {}", text_model_dir.display());
        app_log_info!("  Vision model: {}", vision_model_dir.display());
        app_log_info!("  Whisper model: {}", whisper_model_dir.display());

        let base_url = model_registry_base_url();
        let namespace = model_namespace();
        let text_slug = text_model_slug();
        let vision_slug = vision_model_slug();

        // Helper closure to compose URLs without assuming trailing slashes
        let build_url = |slug: &str, file: &str| -> String {
            format!(
                "{}/{}/{}/{}",
                base_url.trim_end_matches('/'),
                namespace.trim_matches('/'),
                slug.trim_matches('/'),
                file.trim_start_matches('/')
            )
        };

        let models = vec![
            // **Text model files - pulled from the configured registry (defaults to Hugging Face)**
            ModelFile {
                name: "nomic-text-model.onnx".to_string(),
                url: build_url(&text_slug, "model.onnx"),
                destination_path: text_onnx_dir.join("model.onnx"),
            },
            ModelFile {
                name: "nomic-text-config.json".to_string(),
                url: build_url(&text_slug, "config.json"),
                destination_path: text_model_dir.join("config.json"),
            },
            ModelFile {
                name: "nomic-text-tokenizer.json".to_string(),
                url: build_url(&text_slug, "tokenizer.json"),
                destination_path: text_model_dir.join("tokenizer.json"),
            },
            ModelFile {
                name: "nomic-text-tokenizer-config.json".to_string(),
                url: build_url(&text_slug, "tokenizer_config.json"),
                destination_path: text_model_dir.join("tokenizer_config.json"),
            },
            ModelFile {
                name: "nomic-text-special-tokens.json".to_string(),
                url: build_url(&text_slug, "special_tokens_map.json"),
                destination_path: text_model_dir.join("special_tokens_map.json"),
            },
            // **Vision model files - same registry**
            ModelFile {
                name: "nomic-vision-model.onnx".to_string(),
                url: build_url(&vision_slug, "model.onnx"),
                destination_path: vision_onnx_dir.join("model.onnx"),
            },
            ModelFile {
                name: "nomic-vision-preprocessor.json".to_string(),
                url: build_url(&vision_slug, "preprocessor_config.json"),
                destination_path: vision_model_dir.join("preprocessor_config.json"),
            },
            // **Whisper model files - for audio transcription with candle-rs**
            // Using Whisper Base directly from Hugging Face
            ModelFile {
                name: "whisper-base-config.json".to_string(),
                url: "https://huggingface.co/openai/whisper-base/resolve/main/config.json"
                    .to_string(),
                destination_path: whisper_model_dir.join("config.json"),
            },
            ModelFile {
                name: "whisper-base-tokenizer.json".to_string(),
                url: "https://huggingface.co/openai/whisper-base/resolve/main/tokenizer.json"
                    .to_string(),
                destination_path: whisper_model_dir.join("tokenizer.json"),
            },
            ModelFile {
                name: "whisper-base-model.safetensors".to_string(),
                url: "https://huggingface.co/openai/whisper-base/resolve/main/model.safetensors"
                    .to_string(),
                destination_path: whisper_model_dir.join("model.safetensors"),
            },
        ];

        app_log_info!(
            "📦 Configured {} model files for download from {}",
            models.len(),
            base_url
        );
        Ok(models)
    }

    /// Get the path to the Whisper model directory
    pub fn get_whisper_model_path() -> Result<PathBuf> {
        let app_data_dir = path_utils::get_app_data_dir()?;
        let whisper_model_dir = app_data_dir.join("models").join("whisper-base");
        Ok(whisper_model_dir)
    }

    /// Get unified Whisper model status
    pub fn get_whisper_status() -> WhisperStatus {
        // Check download state first
        if let Ok(state) = WHISPER_DOWNLOAD_STATE.lock() {
            match *state {
                WhisperStatus::Downloading => return WhisperStatus::Downloading,
                WhisperStatus::Failed(ref msg) => return WhisperStatus::Failed(msg.clone()),
                _ => {}
            }
        }

        // Check file availability
        match Self::get_whisper_model_path() {
            Ok(whisper_dir) => {
                let model_file = whisper_dir.join("model.safetensors");
                let config_file = whisper_dir.join("config.json");
                let tokenizer_file = whisper_dir.join("tokenizer.json");

                let files_exist =
                    model_file.exists() && config_file.exists() && tokenizer_file.exists();

                if files_exist {
                    app_log_debug!("🎤 Whisper model files available");
                    WhisperStatus::Ready
                } else {
                    app_log_debug!("🎤 Whisper model files missing");
                    WhisperStatus::NotAvailable
                }
            }
            Err(e) => {
                app_log_error!("Failed to get Whisper model path: {}", e);
                WhisperStatus::Failed(format!("Path error: {}", e))
            }
        }
    }

    /// Check which models are missing
    pub fn check_missing_models() -> Result<Vec<ModelFile>> {
        let required_models = Self::get_required_models()?;
        let missing_models: Vec<ModelFile> = required_models
            .into_iter()
            .filter(|model| !model.destination_path.exists())
            .collect();

        app_log_info!("Found {} missing model files", missing_models.len());
        for model in &missing_models {
            app_log_debug!(
                "Missing: {} at {}",
                model.name,
                model.destination_path.display()
            );
        }

        Ok(missing_models)
    }

    /// Check if all required models are available in our simplified structure
    /// **NEW: Checks the simplified models/ directory structure**
    /// This replaces the old .fastembed_cache approach
    pub fn are_models_available() -> bool {
        match Self::check_missing_models() {
            Ok(missing) => {
                let available = missing.is_empty();
                app_log_info!(
                    "📊 Model availability check: {} missing files",
                    missing.len()
                );
                if available {
                    app_log_info!("✅ All required models are available locally");
                } else {
                    app_log_warn!("⚠️ {} model files are missing", missing.len());
                }
                available
            }
            Err(e) => {
                app_log_error!("❌ Failed to check model availability: {}", e);
                false
            }
        }
    }

    /// Clear existing model files (useful for re-downloading corrupted files)
    pub fn clear_existing_models() -> Result<()> {
        let required_models = Self::get_required_models()?;

        for model in required_models {
            if model.destination_path.exists() {
                app_log_info!(
                    "Removing existing model file: {}",
                    model.destination_path.display()
                );
                fs::remove_file(&model.destination_path).map_err(|e| {
                    anyhow!(
                        "Failed to remove {}: {}",
                        model.destination_path.display(),
                        e
                    )
                })?;
            }
        }

        Ok(())
    }

    /// Download a single model file with progress tracking
    pub async fn download_model(
        &self,
        model: &ModelFile,
        progress_callback: impl Fn(DownloadProgress) + Send + Sync,
    ) -> Result<()> {
        app_log_info!("Starting download of {}", model.name);

        // Send initial progress
        progress_callback(DownloadProgress {
            file_name: model.name.clone(),
            downloaded_bytes: 0,
            total_bytes: None,
            percentage: 0.0,
            status: DownloadStatus::Downloading,
        });

        // Start the download
        let response = self
            .client
            .get(&model.url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start download: {}", e))?;

        if !response.status().is_success() {
            let error = format!("HTTP error: {}", response.status());
            progress_callback(DownloadProgress {
                file_name: model.name.clone(),
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                status: DownloadStatus::Failed(error.clone()),
            });
            return Err(anyhow!(error));
        }

        let total_size = response.content_length();
        app_log_debug!("Download content length: {:?}", total_size);

        // Ensure parent directory exists
        if let Some(parent) = model.destination_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create the file
        let mut file = tokio::fs::File::create(&model.destination_path)
            .await
            .map_err(|e| anyhow!("Failed to create file: {}", e))?;

        let mut downloaded_bytes = 0u64;
        let mut stream = response.bytes_stream();

        // **NEW: Progress throttling variables to prevent UI flickering**
        let mut last_progress_time = std::time::Instant::now();
        let mut last_reported_percentage = 0.0f32;
        const PROGRESS_UPDATE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100); // 100ms throttle
        const MIN_PERCENTAGE_CHANGE: f32 = 0.5; // Only update if percentage changed by at least 0.5%

        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| anyhow!("Failed to read chunk: {}", e))?;

            file.write_all(&chunk)
                .await
                .map_err(|e| anyhow!("Failed to write chunk: {}", e))?;

            downloaded_bytes += chunk.len() as u64;

            let percentage = if let Some(total) = total_size {
                (downloaded_bytes as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            // **NEW: Only send progress updates at reasonable intervals to prevent flickering**
            let now = std::time::Instant::now();
            let time_elapsed = now.duration_since(last_progress_time);
            let percentage_changed =
                (percentage - last_reported_percentage).abs() >= MIN_PERCENTAGE_CHANGE;

            if time_elapsed >= PROGRESS_UPDATE_INTERVAL || percentage_changed {
                progress_callback(DownloadProgress {
                    file_name: model.name.clone(),
                    downloaded_bytes,
                    total_bytes: total_size,
                    percentage,
                    status: DownloadStatus::Downloading,
                });

                last_progress_time = now;
                last_reported_percentage = percentage;
            }
        }

        file.flush()
            .await
            .map_err(|e| anyhow!("Failed to flush file: {}", e))?;

        // Verify the file was written correctly
        let file_size = tokio::fs::metadata(&model.destination_path)
            .await
            .map_err(|e| anyhow!("Failed to get file metadata: {}", e))?
            .len();

        if let Some(expected_size) = total_size {
            if file_size != expected_size {
                // Remove corrupted file
                let _ = tokio::fs::remove_file(&model.destination_path).await;
                let error = format!(
                    "File size mismatch: expected {}, got {}",
                    expected_size, file_size
                );
                progress_callback(DownloadProgress {
                    file_name: model.name.clone(),
                    downloaded_bytes: 0,
                    total_bytes: total_size,
                    percentage: 0.0,
                    status: DownloadStatus::Failed(error.clone()),
                });
                return Err(anyhow!(error));
            }
        }

        // Send completion (always send final progress)
        progress_callback(DownloadProgress {
            file_name: model.name.clone(),
            downloaded_bytes,
            total_bytes: total_size,
            percentage: 100.0,
            status: DownloadStatus::Completed,
        });

        app_log_info!(
            "Successfully downloaded {} ({} bytes)",
            model.name,
            file_size
        );
        Ok(())
    }

    /// Download all missing models
    pub async fn download_all_missing_models(
        &self,
        progress_callback: impl Fn(DownloadProgress) + Send + Sync + Clone,
    ) -> Result<()> {
        let missing_models = Self::check_missing_models()?;

        if missing_models.is_empty() {
            app_log_info!("All models are already available");
            return Ok(());
        }

        app_log_info!("Downloading {} missing models", missing_models.len());

        for model in missing_models {
            match self.download_model(&model, progress_callback.clone()).await {
                Ok(_) => {
                    app_log_info!("Successfully downloaded {}", model.name);
                }
                Err(e) => {
                    app_log_error!("Failed to download {}: {}", model.name, e);
                    progress_callback(DownloadProgress {
                        file_name: model.name.clone(),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        percentage: 0.0,
                        status: DownloadStatus::Failed(e.to_string()),
                    });
                    return Err(e);
                }
            }
        }

        app_log_info!("All models downloaded successfully");
        Ok(())
    }
}
