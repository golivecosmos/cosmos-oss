//! Gemma 4 E2B understanding service.
//!
//! Thin wrapper around llama-cpp-4 for local LLM inference. Handles model
//! loading, file description generation, and topic extraction. The
//! EmbeddingService orchestrates when this runs; this service only does
//! inference.
//!
//! ```text
//!   EmbeddingService (orchestrator)
//!        │
//!        ▼
//!   Gemma4Service::describe_file(name, content_preview)
//!        │
//!        ├── model loaded? ──▶ run inference ──▶ return description
//!        ├── model not loaded? ──▶ lazy load ──▶ run inference
//!        └── load failed (OOM)? ──▶ mark disabled ──▶ return None
//! ```

use crate::{app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel, Special};
use llama_cpp_4::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Minimum available system memory (bytes) to attempt loading Gemma 4 E2B.
/// ~1.5GB for Q4_K_M quantization.
const MIN_MEMORY_BYTES: u64 = 1_500_000_000;

/// Maximum tokens to generate per description.
const MAX_DESCRIPTION_TOKENS: usize = 256;

/// Maximum tokens to generate for a topic label.
const MAX_TOPIC_TOKENS: usize = 32;

/// Inference timeout per request in seconds.
const INFERENCE_TIMEOUT_SECS: u64 = 30;

/// Context window size for inference.
const CTX_SIZE: u32 = 4096;

struct LoadedModel {
    backend: LlamaBackend,
    model: LlamaModel,
}

pub struct Gemma4Service {
    loaded: Mutex<Option<LoadedModel>>,
    disabled: AtomicBool,
    model_path: Mutex<Option<PathBuf>>,
}

impl Gemma4Service {
    pub fn new() -> Self {
        Self {
            loaded: Mutex::new(None),
            disabled: AtomicBool::new(false),
            model_path: Mutex::new(None),
        }
    }

    /// Check if the service is available (model exists on disk and not disabled).
    pub fn is_available(&self) -> bool {
        if self.disabled.load(Ordering::Relaxed) {
            return false;
        }
        self.get_model_path().map_or(false, |p| p.exists())
    }

    /// Check if the service has been disabled due to OOM or load failure.
    pub fn is_disabled(&self) -> bool {
        self.disabled.load(Ordering::Relaxed)
    }

    /// Get the expected model file path.
    fn get_model_path(&self) -> Option<PathBuf> {
        if let Ok(guard) = self.model_path.lock() {
            if let Some(ref path) = *guard {
                return Some(path.clone());
            }
        }
        crate::utils::path_utils::get_app_data_dir()
            .ok()
            .map(|dir| dir.join("models").join("gemma-4-e2b.gguf"))
    }

    /// Set a custom model path (for testing or user override).
    pub fn set_model_path(&self, path: PathBuf) {
        if let Ok(mut guard) = self.model_path.lock() {
            *guard = Some(path);
        }
    }

    /// Proactive memory check before loading.
    fn check_memory(&self) -> Result<()> {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let available = sys.available_memory();
        if available < MIN_MEMORY_BYTES {
            return Err(anyhow!(
                "Insufficient memory for Gemma 4: {:.1}GB available, {:.1}GB required",
                available as f64 / 1e9,
                MIN_MEMORY_BYTES as f64 / 1e9
            ));
        }
        app_log_info!(
            "✅ GEMMA4: Memory check passed ({:.1}GB available)",
            available as f64 / 1e9
        );
        Ok(())
    }

    /// Lazy-load the model on first use. Disables service on failure.
    /// Holds the lock through the entire load to prevent concurrent loads.
    pub fn ensure_loaded(&self) -> Result<()> {
        if self.disabled.load(Ordering::Relaxed) {
            return Err(anyhow!("Gemma 4 disabled for this session"));
        }

        let mut guard = self.loaded.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_some() {
            return Ok(());
        }

        // Still holding the lock — no other thread can load concurrently
        if let Err(e) = self.check_memory() {
            app_log_warn!("⚠️ GEMMA4: {}", e);
            self.disabled.store(true, Ordering::Relaxed);
            return Err(e);
        }

        let model_path = self
            .get_model_path()
            .ok_or_else(|| anyhow!("Cannot determine Gemma 4 model path"))?;

        if !model_path.exists() {
            return Err(anyhow!(
                "Gemma 4 model not found at {}. Download it first.",
                model_path.display()
            ));
        }

        app_log_info!("🚀 GEMMA4: Loading model from {}", model_path.display());

        let backend = LlamaBackend::init().map_err(|e| anyhow!("Backend init failed: {}", e))?;
        let model_params = LlamaModelParams::default();

        match LlamaModel::load_from_file(&backend, &model_path, &model_params) {
            Ok(model) => {
                app_log_info!("✅ GEMMA4: Model loaded successfully");
                *guard = Some(LoadedModel { backend, model });
                Ok(())
            }
            Err(e) => {
                app_log_error!("❌ GEMMA4: Failed to load model: {:?}", e);
                self.disabled.store(true, Ordering::Relaxed);
                Err(anyhow!("Failed to load Gemma 4 model: {:?}", e))
            }
        }
    }

    /// Run raw inference with the given prompt and token limit.
    /// Use this for custom prompts (briefings, cluster naming) instead of describe_file().
    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Option<String> {
        let guard = self.loaded.lock().unwrap_or_else(|e| e.into_inner());
        let loaded = guard.as_ref()?;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(CTX_SIZE));

        let mut ctx = match loaded.model.new_context(&loaded.backend, ctx_params) {
            Ok(c) => c,
            Err(e) => {
                app_log_error!("❌ GEMMA4: Context creation failed: {:?}", e);
                return None;
            }
        };

        // Tokenize prompt
        let tokens = match loaded.model.str_to_token(prompt, AddBos::Always) {
            Ok(t) => t,
            Err(e) => {
                app_log_error!("❌ GEMMA4: Tokenization failed: {:?}", e);
                return None;
            }
        };

        // Feed prompt tokens
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch.add(token, i as i32, &[0], is_last).ok()?;
        }

        if let Err(e) = ctx.decode(&mut batch) {
            app_log_error!("❌ GEMMA4: Prompt decode failed: {:?}", e);
            return None;
        }

        // Generate tokens
        let sampler = LlamaSampler::new();

        let mut output = String::new();
        let mut n_cur = tokens.len();
        let start = std::time::Instant::now();

        for _ in 0..max_tokens {
            if start.elapsed().as_secs() > INFERENCE_TIMEOUT_SECS {
                app_log_warn!("⚠️ GEMMA4: Inference timeout after {}s", INFERENCE_TIMEOUT_SECS);
                break;
            }

            let token = sampler.sample(&ctx, -1);

            if loaded.model.is_eog_token(token) {
                break;
            }

            if let Ok(piece) = loaded.model.token_to_str(token, Special::Plaintext) {
                output.push_str(&piece);
            }

            // Prepare next decode
            batch.clear();
            batch.add(token, n_cur as i32, &[0], true).ok()?;
            n_cur += 1;

            if let Err(e) = ctx.decode(&mut batch) {
                app_log_error!("❌ GEMMA4: Decode step failed: {:?}", e);
                break;
            }
        }

        let result = output.trim().to_string();
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Generate a description for a file given its name and a content preview.
    /// Returns None if unavailable or inference fails.
    pub fn describe_file(&self, file_name: &str, content_preview: &str) -> Option<String> {
        if let Err(e) = self.ensure_loaded() {
            app_log_warn!("⚠️ GEMMA4: Skipping description for {}: {}", file_name, e);
            return None;
        }

        let preview: String = content_preview.chars().take(2000).collect();
        let prompt = format!(
            "<start_of_turn>user\nDescribe this file in 1-2 sentences. What is it about? What type of content does it contain?\n\nFilename: {}\nContent preview:\n{}\n<end_of_turn>\n<start_of_turn>model\n",
            file_name, preview
        );

        let start = std::time::Instant::now();
        let result = self.infer(&prompt, MAX_DESCRIPTION_TOKENS);
        app_log_info!(
            "🧠 GEMMA4: describe_file({}) took {}ms",
            file_name,
            start.elapsed().as_millis()
        );
        result
    }

    /// Generate a topic label for a group of file descriptions.
    pub fn extract_topic(&self, descriptions: &[String]) -> Option<String> {
        if let Err(_) = self.ensure_loaded() {
            return None;
        }

        let combined = descriptions
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, d)| format!("{}. {}", i + 1, d))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "<start_of_turn>user\nThese files are grouped together. What is the common topic or category? Reply with just a short label (2-4 words).\n\n{}\n<end_of_turn>\n<start_of_turn>model\n",
            combined
        );

        self.infer(&prompt, MAX_TOPIC_TOKENS)
    }
}
