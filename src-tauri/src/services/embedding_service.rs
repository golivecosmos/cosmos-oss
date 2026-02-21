use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use image::{DynamicImage, GenericImageView};
use serde_json::{json, Value as JsonValue};
use uuid::Uuid;
use crate::services::model_service::ModelService;
use crate::services::sqlite_service::SqliteVectorService;
use crate::services::video_service::VideoService;
use crate::services::drive_service::DriveService;
use crate::services::audio_service::AudioService;
use crate::models::embedding::{ImageVectorDataResponse, VideoFrameMetadata};
use tauri::{AppHandle, Manager};
use crate::{app_log_info, app_log_error, app_log_debug};
use crate::commands::{FailedFileInfo, BulkIndexProgress, VideoProgressInfo, categorize_error};
use crate::services::vector_service::ImageVectorBulkData;

/// **NEW: Result structure for batch indexing operations**
#[derive(Debug, Clone)]
pub struct BatchIndexResult {
    pub successful: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub failed_details: Vec<(String, String)>, // (file_path, error_message)
}

/// Service for managing image embeddings and vector search
pub struct EmbeddingService {
    pub model_service: Arc<ModelService>,
    pub sqlite_service: Arc<SqliteVectorService>,
    pub video_service: Arc<VideoService>,
    pub drive_service: Arc<DriveService>,
    pub audio_service: Option<Arc<tokio::sync::Mutex<AudioService>>>,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(
        model_service: Arc<ModelService>,
        sqlite_service: Arc<SqliteVectorService>,
        drive_service: Arc<DriveService>,
    ) -> Self {
        let video_service = Arc::new(VideoService::new());
        
        // Initialize AudioService 
        let audio_service = {
            let service = AudioService::new();
            app_log_info!("✅ AUDIO: AudioService initialized successfully");
            Some(Arc::new(tokio::sync::Mutex::new(service)))
        };
        
        Self {
            model_service,
            sqlite_service,
            video_service,
            drive_service,
            audio_service,
        }
    }

    /// Check if semantic search is available (models are loaded)
    pub fn is_semantic_search_available(&self) -> bool {
        self.model_service.is_model_loaded()
    }

    /// Search for similar images using an image
    pub async fn search_by_image(&self, query_img: &DynamicImage, limit: i32) -> Result<Vec<ImageVectorDataResponse>> {
        // First preprocess the image
        let processed_img = self.preprocess_image(query_img);
        
        // Generate embedding for the query image
        let embedding = self.model_service.encode_image(&processed_img)?;
        
        // Search for similar vectors using SQLite
        let mut results = self.sqlite_service.search_vectors(&embedding, limit as usize)?;
        
        // Sort by score (ascending - lower is better for cosine distance)
        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }

    /// Process image by normalizing and standardizing it
    pub fn preprocess_image(&self, img: &DynamicImage) -> DynamicImage {
        // Resize to 224x224 for consistency with SigLIP
        let resized = img.resize_exact(224, 224, image::imageops::FilterType::Lanczos3);
        
        // Apply some light preprocessing
        let processed = DynamicImage::ImageRgb8(
            image::imageops::contrast(&resized.to_rgb8(), 1.1)
        );
        
        processed
    }
}

impl EmbeddingService {
    /// Index an image file by generating and storing its embedding
    pub async fn index_image_file(&self, file_path: &str) -> Result<String> {
        let total_start = std::time::Instant::now();
        app_log_debug!("🔄 TIMING: Starting indexing for file: {}", file_path);
        
        // Generate a unique ID for this image
        let id = Uuid::new_v4().to_string();
        
        // Get file information
        let path = Path::new(file_path);
        let is_directory = path.is_dir();
        
        // Extract file metadata
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        // Get the parent path
        let parent_path = path
            .parent()
            .and_then(|p| p.to_str())
            .map(|s| s.to_string());
        
        // Get the MIME type
        let mime_type = if is_directory {
            "inode/directory".to_string()
        } else {
            mime_guess::from_path(file_path)
                .first_or_text_plain()
                .to_string()
        };
        
        // Check if this is a video file
        if self.is_video_file(&mime_type) && self.video_service.is_ffmpeg_available() {
            return self.index_video_file(file_path).await;
        }
        
        // Initialize metadata
        let metadata: JsonValue;
        let embedding: Vec<f32>;
        
        if is_directory {
            // Generate simple metadata for directories
            metadata = json!({
                "is_directory": true,
                "indexed_at": chrono::Utc::now().to_rfc3339()
            });
            
            // Use dummy embedding for directories
            embedding = vec![0.0; 768];
        } else {
            // Try to open the image
            let io_start = std::time::Instant::now();
            match image::open(file_path) {
                Ok(img) => {
                    let io_time = io_start.elapsed();
                    app_log_debug!("⏱️ TIMING: Image I/O took {:.2}ms for {}", io_time.as_millis(), file_path);
                    
                    // Extract image metadata
                    let metadata_start = std::time::Instant::now();
                    metadata = self.extract_image_metadata(&img);
                    let metadata_time = metadata_start.elapsed();
                    app_log_debug!("⏱️ TIMING: Metadata extraction took {:.2}ms", metadata_time.as_millis());
                    
                    // Generate the embedding
                    let inference_start = std::time::Instant::now();
                    match self.model_service.encode_image(&img) {
                        Ok(img_embedding) => {
                            let inference_time = inference_start.elapsed();
                            app_log_debug!("🧠 TIMING: Model inference took {:.2}ms for {} (embedding size: {})", 
                                inference_time.as_millis(), file_path, img_embedding.len());
                            embedding = img_embedding;
                        },
                        Err(e) => {
                            let inference_time = inference_start.elapsed();
                            app_log_error!("❌ TIMING: Model inference failed after {:.2}ms for {}: {}", 
                                inference_time.as_millis(), file_path, e);
                            // Use dummy embedding as fallback
                            embedding = vec![0.0; 768];
                        }
                    }
                },
                Err(e) => {
                    // For non-image files or errors, use simplified metadata and dummy embedding
                    metadata = json!({
                        "error": format!("Failed to open file: {}", e),
                        "indexed_at": chrono::Utc::now().to_rfc3339()
                    });
                    embedding = vec![0.0; 768];
                }
            }
        }
        
        // Combine with filesystem metadata if available
        let enhanced_metadata = if let Ok(fs_metadata) = std::fs::metadata(file_path) {
            let created = fs_metadata.created().ok().map(|t| {
                // Convert SystemTime to DateTime<Utc> safely
                let timestamp = t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
                    .unwrap_or_else(|| chrono::Utc::now())
                    .to_rfc3339()
            });
            
            let modified = fs_metadata.modified().ok().map(|t| {
                // Convert SystemTime to DateTime<Utc> safely
                let timestamp = t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
                    .unwrap_or_else(|| chrono::Utc::now())
                    .to_rfc3339()
            });
            
            let size = fs_metadata.len();
            
            let mut metadata_map = metadata.as_object().cloned().unwrap_or_default();
            if let Some(created) = created {
                metadata_map.insert("fs_created".to_string(), serde_json::Value::String(created));
            }
            if let Some(modified) = modified {
                metadata_map.insert("fs_modified".to_string(), serde_json::Value::String(modified));
            }
            metadata_map.insert("fs_size".to_string(), serde_json::Value::Number(size.into()));
            metadata_map.insert("is_directory".to_string(), serde_json::Value::Bool(is_directory));
            
            serde_json::Value::Object(metadata_map)
        } else {
            metadata
        };
        
        // Detect which drive this file belongs to
        let drive_uuid = if let Some(drive_info) = self.drive_service.get_drive_for_path(file_path).await {
            Some(drive_info.uuid)
        } else {
            None
        };
        
        // Store the embedding in SQLite
        let storage_start = std::time::Instant::now();
        app_log_debug!("💾 STORAGE: Starting SQLite storage for '{}'", file_path);
        
        match self.sqlite_service.store_image_vector_with_drive(
            id.clone(),
            file_path.to_string(),
            parent_path,
            file_name.clone(),
            Some(mime_type),
            embedding.clone(),
            enhanced_metadata.clone(),
            drive_uuid,
        ) {
            Ok(_) => {
                let storage_time = storage_start.elapsed();
                let total_time = total_start.elapsed();
                app_log_debug!("✅ TIMING: SQLite storage took {:.2}ms for {}", storage_time.as_millis(), file_path);
                app_log_debug!("🎯 TIMING: TOTAL file processing took {:.2}ms for {}", total_time.as_millis(), file_path);
            },
            Err(e) => {
                let storage_time = storage_start.elapsed();
                let total_time = total_start.elapsed();
                app_log_error!("❌ TIMING: SQLite storage failed after {:.2}ms (total: {:.2}ms) for {}: {}", 
                    storage_time.as_millis(), total_time.as_millis(), file_path, e);
                return Err(anyhow::anyhow!("Failed to store in SQLite: {}", e));
            }
        }
        
        Ok(id)
    }
    
    /// Index a video file by extracting frames and generating embeddings
    pub async fn index_video_file(&self, video_path: &str) -> Result<String> {
        self.index_video_file_with_mode(video_path, None, true, None).await // Default to fast mode
    }
    
    /// Index a video file with specified performance mode and optional progress reporting (In-Memory)
    pub async fn index_video_file_with_mode(
        &self, 
        video_path: &str, 
        fps: Option<f32>, 
        fast_mode: bool,
        app_handle: Option<AppHandle>
    ) -> Result<String> {
        app_log_info!("🎬 Starting in-memory video indexing for: {}", video_path);
        
        // Check if FFmpeg is available
        if !self.video_service.is_ffmpeg_available() {
            app_log_error!("FFmpeg not available, cannot process video");
            return Err(anyhow::anyhow!("FFmpeg not available, cannot process video"));
        }
        
        // Get video metadata first for better FPS calculation
        let video_metadata = self.video_service.get_video_metadata(video_path)?;
        app_log_info!("📊 Video: {}x{}, {:.1}s duration", 
            video_metadata.width, video_metadata.height, video_metadata.duration);
        
        // Calculate optimal FPS based on mode
        let fps = fps.unwrap_or_else(|| {
            if fast_mode {
                self.calculate_optimal_fps_fast(&video_metadata)
            } else {
                self.calculate_optimal_fps(&video_metadata)
            }
        });
        
        // Enhanced progress callback that reports detailed video processing progress
        let progress_callback = if let Some(ref app_handle) = app_handle {
            let app_handle_clone = app_handle.clone();
            let video_path_clone = video_path.to_string();
            let file_name = video_path.split('/').last().unwrap_or("unknown").to_string();
            
            Some(Box::new(move |progress: crate::services::video_service::VideoProcessingProgress| {
                // Convert VideoProcessingProgress to BulkIndexProgress with video details
                let bulk_progress = BulkIndexProgress {
                    current_file: file_name.clone(),
                    processed: 0, // We'll update this during embedding phase
                    total: 1,
                    status: "processing_video".to_string(),
                    errors: Vec::new(),
                    directory_path: video_path_clone.clone(),
                    failed_files: Vec::new(),
                    video_progress: Some(VideoProgressInfo {
                        current_frame: progress.current_frame,
                        total_frames: progress.total_frames,
                        processing_phase: progress.phase.clone(),
                        video_duration: progress.video_duration,
                        progress_percentage: progress.overall_progress,
                        estimated_time_remaining: progress.time_remaining,
                        current_operation: progress.current_operation.clone(),
                    }),
                    transcription_progress: None,
                };
                
                // Emit progress with retry to reduce transient UI desync.
                let app_handle_for_emit = app_handle_clone.clone();
                tauri::async_runtime::spawn(async move {
                    const MAX_RETRIES: u32 = 3;
                    for attempt in 0..MAX_RETRIES {
                        match app_handle_for_emit.emit_all("bulk_index_progress", &bulk_progress) {
                            Ok(_) => break,
                            Err(e) if attempt < MAX_RETRIES - 1 => {
                                app_log_error!(
                                    "⚠️ VIDEO PROGRESS: Emit failed (attempt {}/{}): {}",
                                    attempt + 1,
                                    MAX_RETRIES,
                                    e
                                );
                                tokio::time::sleep(Duration::from_millis(100 * (attempt + 1) as u64)).await;
                            }
                            Err(e) => {
                                app_log_error!(
                                    "❌ VIDEO PROGRESS: Emit failed after {} attempts: {}",
                                    MAX_RETRIES,
                                    e
                                );
                            }
                        }
                    }
                });
            }) as Box<dyn Fn(crate::services::video_service::VideoProcessingProgress) + Send + Sync>)
        } else {
            None
        };
        
        // Use max resolution for fast mode to speed up processing
        let max_resolution = if fast_mode { Some(512) } else { None };
        
        // Detect which drive this video belongs to
        let video_drive_uuid = if let Some(drive_info) = self.drive_service.get_drive_for_path(video_path).await {
            Some(drive_info.uuid)
        } else {
            None
        };
        
        // Initialize counters for processing
        let embedding_service = self.model_service.clone();
        let sqlite_service = self.sqlite_service.clone();
        
        // Create frame processing callback for in-memory processing
        let video_path_for_callback = video_path.to_string();
        let frame_callback = move |frame: image::DynamicImage, metadata: VideoFrameMetadata| -> Result<()> {
            // Create unique ID for this frame within the video
            let unique_frame_id = format!("{}:frame:{:06}", 
                video_path_for_callback.replace("/", "_").replace(".", "_"), 
                metadata.frame_number
            );
            
            // Format timestamp for display
            let timestamp_str = format!("{:02}:{:02}:{:02}", 
                (metadata.timestamp / 3600.0) as u32,
                ((metadata.timestamp % 3600.0) / 60.0) as u32,
                (metadata.timestamp % 60.0) as u32
            );
            
            // Extract image metadata for this frame
            let (width, height) = frame.dimensions();
            let color_type = match frame {
                image::DynamicImage::ImageLuma8(_) => "grayscale",
                image::DynamicImage::ImageLumaA8(_) => "grayscale_alpha",
                image::DynamicImage::ImageRgb8(_) => "rgb",
                image::DynamicImage::ImageRgba8(_) => "rgba",
                _ => "other"
            };
            
            // Build enhanced metadata including video information
            let enhanced_metadata = json!({
                "source_type": "video_frame",
                "video_path": metadata.video_path,
                "timestamp": metadata.timestamp,
                "timestamp_formatted": timestamp_str,
                "frame_number": metadata.frame_number,
                "video_duration": metadata.video_duration,
                "video_width": metadata.video_width,
                "video_height": metadata.video_height,
                "width": width,
                "height": height,
                "color_type": color_type,
                "aspect_ratio": width as f64 / height as f64,
                "fs_size": 0, // Frame size not applicable for extracted frames
                "is_directory": false,
                "indexed_at": chrono::Utc::now().to_rfc3339(),
            });
            
            // Generate embedding for this frame
            let embedding = embedding_service.encode_image(&frame)
                .map_err(|e| anyhow::anyhow!("Failed to generate embedding: {}", e))?;
            
            // Prepare database storage info
            let video_file_path = metadata.video_path.clone();
            let video_parent_dir = Path::new(&metadata.video_path)
                .parent()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string());
            let video_filename = Path::new(&metadata.video_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown_video".to_string());
            
            // Store in SQLite immediately (inline processing)
            sqlite_service.store_image_vector_with_drive(
                unique_frame_id,
                video_file_path,
                video_parent_dir,
                video_filename,
                Some("video/frame".to_string()),
                embedding,
                enhanced_metadata,
                video_drive_uuid.clone(),
            ).map_err(|e| anyhow::anyhow!("Failed to store frame in database: {}", e))?;
            
            app_log_debug!("✅ VIDEO: Processed and stored frame {} at {:.1}s", 
                metadata.frame_number, metadata.timestamp);
            
            Ok(())
        };
        
        // Process video using in-memory method
        let total_frames = self.video_service.process_video_frames_in_memory(
            video_path, 
            fps, 
            false, // Scene detection disabled for consistency with current behavior
            max_resolution,
            frame_callback,
            progress_callback
        ).await?;
        
        let successful_embeds = total_frames;
        
        let video_id = format!("video:{}", 
            video_path.replace("/", "_").replace(".", "_")
        );
        
        app_log_info!("🎬 In-memory video indexing complete: {} | {} embeddings generated from {} frames", 
            video_path, successful_embeds, total_frames);
        
        Ok(video_id)
    }
    
    /// Calculate ultra-fast frame extraction rate - prioritizes speed over coverage
    fn calculate_optimal_fps_fast(&self, metadata: &crate::services::video_service::VideoMetadata) -> f32 {
        let duration = metadata.duration;
        let resolution = metadata.width as f64 * metadata.height as f64;
        
        // Ultra-aggressive sampling for maximum speed
        match duration {
            d if d <= 5.0 => {
                // Very short videos: minimal sampling
                0.5  // 1 frame every 2 seconds regardless of resolution
            },
            d if d <= 30.0 => {
                // Short videos: very sparse
                if resolution > 1920.0 * 1080.0 {
                    0.2  // High resolution: 1 frame every 5 seconds
                } else {
                    0.33 // Lower resolution: 1 frame every 3 seconds
                }
            },
            d if d <= 300.0 => {
                // Medium videos: ultra sparse
                if resolution > 1920.0 * 1080.0 {
                    0.1  // High resolution: 1 frame every 10 seconds
                } else {
                    0.15 // Lower resolution: 1 frame every ~7 seconds
                }
            },
            _ => {
                // Long videos: minimal coverage
                if resolution > 1920.0 * 1080.0 {
                    0.033 // High resolution: 1 frame every 30 seconds
                } else {
                    0.05  // Lower resolution: 1 frame every 20 seconds
                }
            }
        }
    }
    
    /// Calculate optimal frame extraction rate based on video characteristics
    fn calculate_optimal_fps(&self, metadata: &crate::services::video_service::VideoMetadata) -> f32 {
        let duration = metadata.duration;
        let resolution = metadata.width as f64 * metadata.height as f64;
        
        // More aggressive frame rate optimization - prioritize speed over coverage
        match duration {
            d if d <= 10.0 => {
                // Very short videos: sample moderately
                if resolution > 1920.0 * 1080.0 {
                    1.0  // High resolution: 1 fps (was 2.0)
                } else {
                    1.5  // Lower resolution: 1.5 fps (was 3.0)
                }
            },
            d if d <= 30.0 => {
                // Short videos: reduced sampling
                if resolution > 1920.0 * 1080.0 {
                    0.5  // High resolution: 0.5 fps (was 2.0)
                } else {
                    0.75 // Lower resolution: 0.75 fps (was 3.0)
                }
            },
            d if d <= 300.0 => {
                // Medium videos (5 minutes): much sparser sampling
                if resolution > 1920.0 * 1080.0 {
                    0.25 // High resolution: 0.25 fps (was 1.0)
                } else {
                    0.33 // Lower resolution: 0.33 fps (was 1.5)
                }
            },
            d if d <= 1800.0 => {
                // Long videos (30 minutes): very sparse sampling
                if resolution > 1920.0 * 1080.0 {
                    0.1  // High resolution: 0.1 fps - 1 frame every 10 seconds (was 0.5)
                } else {
                    0.15 // Lower resolution: 0.15 fps - 1 frame every ~7 seconds (was 0.75)
                }
            },
            _ => {
                // Very long videos: extremely sparse sampling
                if resolution > 1920.0 * 1080.0 {
                    0.05 // High resolution: 0.05 fps - 1 frame every 20 seconds (was 0.25)
                } else {
                    0.1  // Lower resolution: 0.1 fps - 1 frame every 10 seconds (was 0.33)
                }
            }
        }
    }
    
    /// Check if a MIME type represents a video file
    fn is_video_file(&self, mime_type: &str) -> bool {
        mime_type.starts_with("video/")
    }

    fn categorize_image_open_error(err: &image::ImageError) -> &'static str {
        match err {
            image::ImageError::IoError(io_err) => match io_err.kind() {
                std::io::ErrorKind::PermissionDenied => "PERMISSION_DENIED",
                std::io::ErrorKind::NotFound => "FILE_NOT_FOUND",
                std::io::ErrorKind::WouldBlock => "FILE_LOCKED",
                _ => "IO_ERROR",
            },
            image::ImageError::Unsupported(_) => "UNSUPPORTED_FORMAT",
            image::ImageError::Decoding(_) => "CORRUPTED_FILE",
            _ => "UNKNOWN_ERROR",
        }
    }
    
    /// Extract metadata from an image
    fn extract_image_metadata(&self, img: &DynamicImage) -> JsonValue {
        let (width, height) = img.dimensions();
        let color_type = match img {
            DynamicImage::ImageLuma8(_) => "grayscale",
            DynamicImage::ImageLumaA8(_) => "grayscale_alpha",
            DynamicImage::ImageRgb8(_) => "rgb",
            DynamicImage::ImageRgba8(_) => "rgba",
            _ => "other"
        };

        json!({
            "dimensions": {
                "width": width,
                "height": height
            },
            "color_type": color_type,
            "aspect_ratio": width as f64 / height as f64
        })
    }
    
    /// Get text embedding for benchmarking purposes
    pub async fn get_text_embedding_for_benchmark(&self, text: &str) -> Result<(Vec<f32>, String)> {
        // Apply the prompt template
        let enhanced_query = self.model_service.apply_prompt_template(text);
        
        // Generate text embedding
        let embedding = self.model_service.encode_text(&enhanced_query)?;
        
        Ok((embedding, enhanced_query))
    }
    
    

    /// **NEW: Batch index multiple image files at once**
    pub async fn index_image_files_batch(&self, file_paths: Vec<String>) -> Result<BatchIndexResult> {
        if file_paths.is_empty() {
            return Ok(BatchIndexResult {
                successful: 0,
                failed: 0,
                errors: Vec::new(),
                failed_details: Vec::new(),
            });
        }
        
        app_log_info!("🚀 BATCH INDEX: Processing batch of {} files", file_paths.len());
        
        let mut batch_data = Vec::new();
        let mut errors = Vec::new();
        let mut failed_files = Vec::new();
        let mut failed_details: Vec<(String, String)> = Vec::new();
        
        // Process each file and collect embeddings
        for file_path in file_paths {
            match self.process_single_file_for_batch(&file_path).await {
                Ok(data) => {
                    batch_data.push(data);
                }
                Err(e) => {
                    let error_msg = format!("Failed to process {}: {}", file_path, e);
                    app_log_error!("❌ BATCH INDEX: {}", error_msg);
                    errors.push(error_msg);
                    failed_details.push((file_path.clone(), e.to_string()));
                    
                    let file_name = std::path::Path::new(&file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    failed_files.push(FailedFileInfo {
                        name: file_name,
                        path: file_path,
                        error: e.to_string(),
                        error_type: categorize_error(&e.to_string()).to_string(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
        }
        
        let successful_embeddings = batch_data.len();
        
        // Bulk insert all successful embeddings
        if !batch_data.is_empty() {
            match self.sqlite_service.store_image_vectors_bulk(batch_data.clone()) {
                Ok(stored_count) => {
                    app_log_info!("✅ BATCH INDEX: Successfully stored {} embeddings", stored_count);
                    if stored_count < successful_embeddings {
                        let shortfall = successful_embeddings - stored_count;
                        let mismatch_error = format!(
                            "Bulk storage mismatch: generated {} embeddings but stored {}",
                            successful_embeddings, stored_count
                        );
                        app_log_error!("❌ BATCH INDEX: {}", mismatch_error);
                        errors.push(mismatch_error);

                        return Ok(BatchIndexResult {
                            successful: stored_count,
                            failed: failed_files.len() + shortfall,
                            errors,
                            failed_details,
                        });
                    }
                }
                Err(e) => {
                    app_log_error!("❌ BATCH INDEX: Failed to bulk store embeddings: {}", e);
                    // Fall back to individual storage to avoid losing generated embeddings.
                    let bulk_error = format!("Bulk storage failed, falling back to individual writes: {}", e);
                    errors.push(bulk_error);

                    let mut individually_stored = 0usize;
                    for item in batch_data {
                        let item_file_path = item.file_path.clone();
                        match self.sqlite_service.store_image_vector_with_drive(
                            item.id,
                            item.file_path,
                            item.parent_file_path,
                            item.file_name,
                            item.mime_type,
                            item.embedding,
                            item.metadata,
                            item.drive_uuid,
                        ) {
                            Ok(_) => {
                                individually_stored += 1;
                            }
                            Err(store_err) => {
                                let store_error_msg =
                                    format!("Failed to store {} after bulk failure: {}", item_file_path, store_err);
                                app_log_error!("❌ BATCH INDEX: {}", store_error_msg);
                                errors.push(store_error_msg);
                                failed_details.push((item_file_path, store_err.to_string()));
                            }
                        }
                    }

                    return Ok(BatchIndexResult {
                        successful: individually_stored,
                        failed: failed_files.len() + (successful_embeddings - individually_stored),
                        errors,
                        failed_details,
                    });
                }
            }
        }
        
        app_log_info!("📊 BATCH INDEX: Completed - {} successful, {} failed", 
            successful_embeddings, failed_files.len());
        
        Ok(BatchIndexResult {
            successful: successful_embeddings,
            failed: failed_files.len(),
            errors,
            failed_details,
        })
    }
    
    /// **NEW: Process a single file for batch operations**
    async fn process_single_file_for_batch(&self, file_path: &str) -> Result<ImageVectorBulkData> {
        
        // Generate a unique ID for this image
        let id = Uuid::new_v4().to_string();
        
        // Get file information
        let path = Path::new(file_path);
        
        // Extract file metadata
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        // Get the parent path
        let parent_path = path
            .parent()
            .map(|p| p.to_string_lossy().to_string());
        
        // Try to determine the MIME type
        let mime_type = mime_guess::from_path(path)
            .first()
            .map(|mime| mime.to_string());
        
        // Load and process the image
        let img = match image::open(path) {
            Ok(img) => img,
            Err(e) => {
                let category = Self::categorize_image_open_error(&e);
                return Err(anyhow::anyhow!(
                    "[{}] Failed to load image {}: {}",
                    category,
                    file_path,
                    e
                ));
            }
        };
        
        // Preprocess the image
        let processed_img = self.preprocess_image(&img);
        
        // Generate embedding
        let embedding = match self.model_service.encode_image(&processed_img) {
            Ok(emb) => emb,
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to generate embedding for {}: {}", file_path, e));
            }
        };
        
        // Extract image metadata
        let metadata = self.extract_image_metadata(&img);
        
        // Add file system metadata
        let mut enhanced_metadata = metadata;
        if let Ok(file_metadata) = std::fs::metadata(path) {
            enhanced_metadata["fs_size"] = json!(file_metadata.len());
            if let Ok(created) = file_metadata.created() {
                if let Ok(created_time) = created.duration_since(std::time::UNIX_EPOCH) {
                    enhanced_metadata["fs_created"] = json!(created_time.as_secs());
                }
            }
        }
        
        // Detect which drive this file belongs to
        let drive_uuid = if let Some(drive_info) = self.drive_service.get_drive_for_path(file_path).await {
            Some(drive_info.uuid)
        } else {
            None
        };
        
        Ok(ImageVectorBulkData {
            id,
            file_path: file_path.to_string(),
            parent_file_path: parent_path,
            file_name,
            mime_type,
            embedding,
            metadata: enhanced_metadata,
            drive_uuid,
        })
    }
} 
