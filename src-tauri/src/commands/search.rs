use tauri::State;
use crate::AppState;
use crate::models::embedding::ImageVectorDataResponse;
use crate::app_log_info;
use crate::app_log_error;
use crate::app_log_warn;
use serde_json::json;
use base64::Engine;
use image;

// **NEW: Benchmark result structure for comparing backends**
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResult {
    pub backend: String,
    pub query_time_ms: f64,
    pub result_count: usize,
    pub results: Vec<ImageVectorDataResponse>,
    pub memory_usage_mb: Option<f64>,
    pub index_size_mb: Option<f64>,
    pub error: Option<String>,
}

/// Search for similar images using text
#[tauri::command]
pub async fn search_semantic(query: String, state: State<'_, AppState>) -> Result<Vec<ImageVectorDataResponse>, String> {
    app_log_info!("🔍 SEMANTIC SEARCH: Starting search for query: '{}'", query);

    // Validate search query
    if query.trim().is_empty() {
        app_log_warn!("Empty search query provided");
        return Err("Search query cannot be empty".to_string());
    }

    // Check if semantic search is available
    if !state.embedding_service.is_semantic_search_available() {
        app_log_warn!("Semantic search not available - models not loaded");
        return Err("Search models are not loaded. Please wait for model download to complete.".to_string());
    }

    app_log_info!("🗄️ Using SQLite for vector search");

    // Get text embedding for SQLite search
    match state.embedding_service.get_text_embedding_for_benchmark(&query).await {
        Ok(embedding) => {
            match state.sqlite_service.search_vectors(&embedding.0, 60) {
                Ok(results) => {
                    app_log_info!("✅ SEARCH RESULTS: Got {} results", results.len());

                    // Log scores for debugging
                    for (i, result) in results.iter().enumerate().take(5) {
                        app_log_info!("🔍 RESULT {}: file='{}', score={:.4}, is_video_frame={}",
                            i + 1, result.file_path, result.score, result.timestamp.is_some());
                    }

                    Ok(results)
                },
                Err(e) => {
                    app_log_error!("❌ SEARCH FAILED: {}", e);
                    Err(format!("Search failed: {}", e))
                }
            }
        },
        Err(e) => {
            app_log_error!("❌ EMBEDDING GENERATION FAILED: {}", e);
            Err(format!("Failed to generate embedding: {}", e))
        }
    }
}

/// Search for similar images using visual similarity
#[tauri::command]
pub async fn search_visual(image_data: String, state: State<'_, AppState>) -> Result<Vec<ImageVectorDataResponse>, String> {
    app_log_info!("🔍 VISUAL SEARCH: Starting visual search");

    // Validate base64 data
    if image_data.trim().is_empty() {
        app_log_warn!("Empty image data provided");
        return Err("Image data cannot be empty".to_string());
    }

    // Check if semantic search is available
    if !state.embedding_service.is_semantic_search_available() {
        app_log_warn!("Visual search not available - models not loaded");
        return Err("Search models are not loaded. Please wait for model download to complete.".to_string());
    }

    // Decode base64 image
    let image_bytes = match base64::engine::general_purpose::STANDARD.decode(&image_data) {
        Ok(bytes) => bytes,
        Err(e) => {
            app_log_error!("Failed to decode base64 image: {}", e);
            return Err("Failed to decode image data".to_string());
        }
    };

    // Load image
    let img = match image::load_from_memory(&image_bytes) {
        Ok(img) => img,
        Err(e) => {
            app_log_error!("Failed to load image: {}", e);
            return Err("Failed to load image".to_string());
        }
    };

    app_log_info!("🖼️ VISUAL SEARCH: Image loaded successfully, dimensions: {}x{}", img.width(), img.height());

    // Perform visual search using SQLite backend
    match state.embedding_service.search_by_image(&img, 20).await {
        Ok(results) => {
            app_log_info!("✅ VISUAL SEARCH: Got {} results", results.len());

            // Log scores for debugging
            for (i, result) in results.iter().enumerate().take(5) {
                app_log_info!("🔍 RESULT {}: file='{}', score={:.4}, is_video_frame={}",
                    i + 1, result.file_path, result.score, result.is_video_frame());
            }

            // **CHANGED: Return individual frames instead of grouping them**
            // This allows users to see multiple matching frames from videos

            // Sort by score (ascending - lower cosine distance = better similarity)
            let mut final_results = results;
            final_results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));

            app_log_info!("✅ RETURNING {} INDIVIDUAL RESULTS (including video frames)", final_results.len());

            Ok(final_results)
        },
        Err(e) => {
            app_log_error!("❌ VISUAL SEARCH FAILED: {}", e);
            Err(format!("Visual search failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn check_search_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    app_log_info!("🔍 STATUS CHECK: Checking search system status");

    let embedding_service = &state.embedding_service;
    let sqlite_service = &state.sqlite_service;

    // Check if semantic search is available
    let model_loaded = embedding_service.is_semantic_search_available();
    app_log_info!("📊 MODEL STATUS: Loaded = {}", model_loaded);

    // Get SQLite stats
    let sqlite_stats = match sqlite_service.get_stats() {
        Ok(stats) => {
            app_log_info!("📊 SQLITE STATUS: {}", serde_json::to_string_pretty(&stats).unwrap_or_default());
            stats
        },
        Err(e) => {
            app_log_error!("📊 SQLITE ERROR: Failed to get stats - {}", e);
            json!({})
        }
    };

    // Get indexed count from SQLite
    let indexed_count = match sqlite_service.get_image_count() {
        Ok(count) => {
            app_log_info!("📊 INDEX STATUS: {} files indexed", count);
            count
        },
        Err(e) => {
            app_log_error!("📊 INDEX ERROR: Failed to get count - {}", e);
            0
        }
    };

    let status = serde_json::json!({
        "model_loaded": model_loaded,
        "indexed_count": indexed_count,
        "sqlite_stats": sqlite_stats,
        "status": if model_loaded { "ready" } else { "not_ready" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    app_log_info!("📊 OVERALL STATUS: {:?}", status);

    Ok(status)
}
