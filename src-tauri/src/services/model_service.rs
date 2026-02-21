use anyhow::Result;
use std::sync::Arc;
use std::sync::Mutex;

use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use crate::models::{EmbeddingModel, nomic::NomicModel};

// Configuration for model selection
#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Nomic,
}

impl Default for ModelType {
    fn default() -> Self {
        ModelType::Nomic  // Switch to Nomic for testing
    }
}

/// Service for managing ML models
pub struct ModelService {
    active_model: Arc<Mutex<Option<Box<dyn EmbeddingModel>>>>,
    model_type: ModelType,
}

impl ModelService {
    /// Create a new model service with default CLIP model
    pub fn new() -> Self {
        Self::new_with_model_type(ModelType::default())
    }

    /// Create a new model service with specified model type
    pub fn new_with_model_type(model_type: ModelType) -> Self {
        app_log_info!("Initializing ModelService with model type: {:?}...", model_type);

        let model = Self::load_model(&model_type).unwrap_or_else(|e| {
            app_log_error!("Failed to load model during initialization: {}", e);
            None
        });

        if model.is_some() {
            app_log_info!("Model loaded successfully");
        } else {
            app_log_error!("Model not available - semantic search disabled");
        }

        Self {
            active_model: Arc::new(Mutex::new(model)),
            model_type,
        }
    }

    /// Check if any model is loaded
    pub fn is_model_loaded(&self) -> bool {
        match self.active_model.lock() {
            Ok(guard) => guard.is_some(),
            Err(_) => {
                app_log_warn!("Model lock is poisoned, attempting to recover...");
                match self.active_model.try_lock() {
                    Ok(guard) => guard.is_some(),
                    Err(_) => {
                        app_log_error!("Could not recover from poisoned lock");
                        false
                    }
                }
            }
        }
    }

    /// Reload the current model type (useful after downloading new models)
    pub async fn reload_model(&self) -> Result<()> {
        app_log_info!("Reloading {:?} model...", self.model_type);

        // Give a small delay to ensure files are fully written
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Log current model directory and file status
        self.log_model_status().await;

        let model = Self::load_model(&self.model_type)?
            .ok_or_else(|| anyhow::anyhow!("Failed to load {:?} model", self.model_type))?;

        // Update the stored model - handle poisoned lock gracefully
        match self.active_model.lock() {
            Ok(mut model_guard) => {
                *model_guard = Some(model);
                app_log_info!("{:?} model reloaded successfully", self.model_type);
                Ok(())
            },
            Err(_) => {
                app_log_error!("Model lock is poisoned during reload");
                Err(anyhow::anyhow!("Failed to reload model - lock is poisoned"))
            }
        }
    }

    /// Legacy method for backward compatibility
    pub async fn reload_clip_model(&self) -> Result<()> {
        self.reload_model().await
    }

    /// Log detailed model status for debugging
    async fn log_model_status(&self) {
        match self.model_type {
            ModelType::Nomic => {
                // Model status logging is now handled within NomicModel itself
                app_log_info!("Checking Nomic model availability...");
            }
        }
    }

    /// Load a model of the specified type
    fn load_model(model_type: &ModelType) -> Result<Option<Box<dyn EmbeddingModel>>> {
        app_log_debug!("Attempting to load {:?} model...", model_type);

        match model_type {
            ModelType::Nomic => Self::load_nomic_model(),
        }
    }

    /// Load Nomic model from the models directory
    fn load_nomic_model() -> Result<Option<Box<dyn EmbeddingModel>>> {
        app_log_info!("🔄 Loading Nomic Embed models...");

        // Try to create the Nomic model - it handles its own directory structure
        match NomicModel::new() {
            Ok(model) => {
                app_log_info!("✅ Successfully loaded Nomic models");
                Ok(Some(Box::new(model)))
            },
            Err(e) => {
                app_log_error!("❌ Failed to load Nomic models: {}", e);
                Err(anyhow::anyhow!("Failed to load Nomic models: {}", e))
            }
        }
    }

    /// Generate text embedding from input text using the active model
    pub fn encode_text(&self, text: &str) -> Result<Vec<f32>> {
        let model_guard = self.active_model.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire model lock"))?;

        let model = model_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;

        model.encode_text(text)
    }

    /// Generate image embedding from input image using the active model
    pub fn encode_image(&self, img: &image::DynamicImage) -> Result<Vec<f32>> {
        let model_guard = self.active_model.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire model lock"))?;

        let model = model_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No model loaded"))?;

        model.encode_image(img)
    }

    /// Apply prompt template to improve text embeddings (model-specific)
    pub fn apply_prompt_template(&self, query: &str) -> String {
        match self.model_type {
            ModelType::Nomic => Self::apply_nomic_template(query),
        }
    }

    /// Apply Nomic prompt template to improve text embeddings
    pub fn apply_nomic_template(query: &str) -> String {
        // Nomic models require the "search_query:" prefix for text queries
        // This is handled directly in the NomicModel::encode_text method
        // so we don't need to add it here to avoid double-prefixing
        query.to_string()
    }
}
