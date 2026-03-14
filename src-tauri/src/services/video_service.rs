use crate::models::embedding::VideoFrameMetadata;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use std::env;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::Instant;

/// Progress information for video processing
#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoProcessingProgress {
    pub phase: String,             // "metadata", "extraction", "processing", "complete"
    pub current_frame: usize,      // Current frame being processed
    pub total_frames: usize,       // Total number of frames
    pub processed_frames: usize,   // Successfully processed frames
    pub phase_progress: f64,       // Progress within current phase (0-100)
    pub overall_progress: f64,     // Overall progress (0-100)
    pub current_operation: String, // Description of current operation
    pub fps: f32,                  // Frame rate being used
    pub video_duration: f64,       // Total video duration in seconds
    pub estimated_frames: usize,   // Estimated total frames
    pub processing_speed: f64,     // Frames processed per second
    pub time_remaining: f64,       // Estimated time remaining in seconds
}

/// Service for processing video files
pub struct VideoService {
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
}

impl VideoService {
    /// Create a new video service instance
    pub fn new() -> Self {
        app_log_info!("🎥 Initializing video service...");

        // Get current working directory
        if let Ok(cwd) = std::env::current_dir() {
            app_log_info!("📁 Current working directory: {:?}", cwd);
        } else {
            app_log_error!("❌ Failed to get current working directory");
        }

        // Try to find bundled FFmpeg and FFprobe
        app_log_info!("🔍 Searching for FFmpeg and FFprobe binaries...");
        let (ffmpeg_path, ffprobe_path) = Self::find_ffmpeg_executables();

        // Log the results
        match (&ffmpeg_path, &ffprobe_path) {
            (Some(f), Some(p)) => {
                app_log_info!("✅ Found both FFmpeg binaries:");
                app_log_info!("  FFmpeg: {:?}", f);
                app_log_info!("  FFprobe: {:?}", p);

                // Check if they're executable
                if let Ok(metadata) = std::fs::metadata(f) {
                    app_log_info!("  FFmpeg permissions: {:?}", metadata.permissions());
                }
                if let Ok(metadata) = std::fs::metadata(p) {
                    app_log_info!("  FFprobe permissions: {:?}", metadata.permissions());
                }
            }
            (Some(f), None) => {
                app_log_error!("❌ Found FFmpeg but missing FFprobe:");
                app_log_info!("  FFmpeg: {:?}", f);
            }
            (None, Some(p)) => {
                app_log_error!("❌ Found FFprobe but missing FFmpeg:");
                app_log_info!("  FFprobe: {:?}", p);
            }
            (None, None) => {
                app_log_error!("❌ Neither FFmpeg nor FFprobe found");
            }
        }

        Self {
            ffmpeg_path,
            ffprobe_path,
        }
    }

    /// Check if FFmpeg is available
    pub fn is_ffmpeg_available(&self) -> bool {
        let has_both = self.ffmpeg_path.is_some() && self.ffprobe_path.is_some();
        app_log_info!(
            "🔍 Checking FFmpeg availability: {}",
            if has_both {
                "✅ available"
            } else {
                "❌ not available"
            }
        );
        has_both
    }

    /// Find FFmpeg executables with clean environment-specific paths
    fn find_ffmpeg_executables() -> (Option<PathBuf>, Option<PathBuf>) {
        // Log build mode
        #[cfg(debug_assertions)]
        {
            app_log_info!("🔧 BUILD MODE: Development - searching in src-tauri/bin/");
        }

        #[cfg(not(debug_assertions))]
        {
            app_log_info!("🚀 BUILD MODE: Production - searching in Resources/");
        }

        // Helper function to resolve a single canonical bundled path.
        let bundled_binary_path = |binary_name: &str| -> Option<PathBuf> {
            #[cfg(debug_assertions)]
            {
                Some(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("bin")
                        .join(binary_name),
                )
            }

            #[cfg(not(debug_assertions))]
            {
                match env::current_exe() {
                    Ok(exe_path) => exe_path
                        .parent()
                        .map(|exe_dir| exe_dir.join("../Resources/bin").join(binary_name)),
                    Err(e) => {
                        app_log_error!("❌ Failed to get current executable path: {}", e);
                        None
                    }
                }
            }
        };

        // Helper function to validate bundled binary path.
        let find_binary = |binary_name: &str| -> Option<PathBuf> {
            let Some(path) = bundled_binary_path(binary_name) else {
                app_log_error!("❌ Unable to resolve bundled path for {}", binary_name);
                return None;
            };

            app_log_info!("🔍 Checking {}: {:?}", binary_name, path);
            if path.exists() {
                app_log_info!("✅ Found {} at: {:?}", binary_name, path);
                Some(path)
            } else {
                app_log_info!("❌ {} not found at: {:?}", binary_name, path);
                app_log_error!("❌ {} not found in bundled location", binary_name);
                None
            }
        };

        let ffmpeg_path = find_binary("ffmpeg");
        let ffprobe_path = find_binary("ffprobe");

        (ffmpeg_path, ffprobe_path)
    }

    /// Get metadata about a video file using FFprobe
    pub fn get_video_metadata(&self, video_path: &str) -> Result<VideoMetadata> {
        // Ensure FFprobe is available
        let ffprobe = match &self.ffprobe_path {
            Some(path) => path,
            None => {
                return Err(anyhow!(
                    "Bundled FFprobe not found, cannot get video metadata"
                ))
            }
        };

        // Use FFprobe to get video information (software mode first, more reliable)
        let mut cmd = Command::new(ffprobe);
        cmd.args([
            "-v",
            "error", // Minimal logging
            "-select_streams",
            "v:0", // Only first video stream
            "-show_entries",
            "stream=width,height,duration,codec_name,bit_rate,avg_frame_rate", // Get more metadata efficiently
            "-of",
            "json", // JSON output for easy parsing
            video_path,
        ]);

        let output = cmd.output()?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to get video metadata: {}", error));
        }

        // Parse the JSON output
        let output_str = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&output_str)?;

        // Extract the stream information
        let streams = json["streams"]
            .as_array()
            .ok_or_else(|| anyhow!("No streams found in video"))?;

        if streams.is_empty() {
            return Err(anyhow!("No video streams found"));
        }

        let stream = &streams[0];

        // Extract metadata
        let width = stream["width"].as_u64().unwrap_or(0) as u32;
        let height = stream["height"].as_u64().unwrap_or(0) as u32;
        let duration = stream["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .unwrap_or(0.0);
        let codec = stream["codec_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(VideoMetadata {
            width,
            height,
            duration,
            codec,
        })
    }

    /// Calculate a simple hash for frame similarity detection
    fn calculate_frame_hash(img: &image::DynamicImage) -> u64 {
        // Resize to a small size for fast comparison
        let small = img.resize_exact(8, 8, image::imageops::FilterType::Triangle);
        let gray = small.to_luma8();

        // Calculate average brightness
        let avg: u32 = gray.as_raw().iter().map(|&p| p as u32).sum::<u32>() / 64;

        // Create hash based on pixels above/below average
        let mut hash = 0u64;
        for (i, &pixel) in gray.as_raw().iter().enumerate() {
            if pixel as u32 > avg {
                hash |= 1u64 << (i % 64);
            }
        }

        hash
    }

    /// Calculate similarity between two hashes (0.0 = completely different, 1.0 = identical)
    fn calculate_hash_similarity(hash1: u64, hash2: u64) -> f64 {
        let xor = hash1 ^ hash2;
        let different_bits = xor.count_ones();
        1.0 - (different_bits as f64 / 64.0)
    }

    /// Analyze video characteristics to determine optimal sampling strategy
    fn analyze_video_characteristics(
        &self,
        metadata: &VideoMetadata,
        video_path: &str,
    ) -> Result<VideoAnalysis> {
        let aspect_ratio = metadata.width as f32 / metadata.height as f32;
        let is_portrait = aspect_ratio < 1.0;
        let is_square_ish = (aspect_ratio - 1.0).abs() < 0.2; // Close to 1:1 ratio
        let is_widescreen = aspect_ratio > 1.7;

        // Get file size for additional heuristics
        let file_size_mb = std::fs::metadata(video_path)
            .map(|meta| meta.len() / (1024 * 1024))
            .unwrap_or(0);

        // **IMPROVED: More sophisticated content type detection**
        let content_type = if is_portrait || is_square_ish {
            // Portrait or square videos are very likely social media/talking heads
            "talking_head"
        } else if metadata.duration < 120.0 && !is_widescreen {
            // Short videos that aren't widescreen are likely talking heads or simple content
            "talking_head"
        } else if metadata.duration > 1800.0 && file_size_mb < (metadata.duration as u64 / 60) * 10
        {
            // Long videos with low bitrate (< 10MB per minute) are likely static content
            "static"
        } else if file_size_mb > (metadata.duration as u64 / 60) * 50 && metadata.width >= 1920 {
            // High bitrate + high resolution suggests action content
            "action"
        } else if metadata.duration < 60.0 && file_size_mb < 20 {
            // Very short, small files are likely talking heads or simple content
            "talking_head"
        } else if is_widescreen && metadata.duration > 300.0 {
            // Widescreen longer videos might be action content
            "action"
        } else {
            "mixed"
        };

        // **IMPROVED: More nuanced motion level estimation**
        let motion_level = match content_type {
            "talking_head" => {
                // Even within talking heads, consider duration and file size
                if metadata.duration > 600.0 || file_size_mb < (metadata.duration as u64 / 60) * 5 {
                    0.1 // Very low motion for long or low-bitrate talking heads
                } else {
                    0.2 // Low motion for typical talking heads
                }
            }
            "static" => 0.05, // Very low motion
            "action" => {
                // Consider resolution and bitrate for action content
                if file_size_mb > (metadata.duration as u64 / 60) * 30 {
                    0.9 // High motion for high-bitrate action
                } else {
                    0.7 // Medium-high motion for typical action
                }
            }
            _ => 0.5, // Medium motion for mixed content
        };

        let analysis = VideoAnalysis {
            content_type: content_type.to_string(),
            motion_level,
        };

        Ok(analysis)
    }

    /// Calculate smart sampling rate based on video analysis
    fn calculate_smart_sampling_rate(&self, analysis: &VideoAnalysis, requested_fps: f32) -> f32 {
        let base_fps = match analysis.content_type.as_str() {
            "talking_head" => 0.2, // **IMPROVED: Even more aggressive - 1 frame every 5 seconds**
            "static" => 0.05,      // **IMPROVED: 1 frame every 20 seconds for static content**
            "action" => 1.5,       // 1.5 frames per second for action
            "mixed" => 0.8,        // 0.8 frames per second for mixed content
            _ => 1.0,
        };

        // Adjust based on motion level (lower motion = fewer frames needed)
        let motion_adjusted = base_fps * (0.5 + analysis.motion_level * 0.5);

        // For talking heads, be even more conservative
        let final_adjusted = if analysis.content_type == "talking_head" {
            motion_adjusted * 0.5 // **NEW: Additional 50% reduction for talking heads**
        } else {
            motion_adjusted
        };

        // Respect user's requested FPS as upper bound, but prioritize our optimization
        let final_fps = final_adjusted.min(requested_fps);

        // Ensure minimum viable sampling (but allow very low rates for static content)
        let min_fps = if analysis.content_type == "static" {
            0.02
        } else {
            0.1
        };
        let result = final_fps.max(min_fps);

        result
    }

    /// Get similarity threshold based on content type
    fn get_similarity_threshold(&self, analysis: &VideoAnalysis) -> f64 {
        let threshold = match analysis.content_type.as_str() {
            "talking_head" => 0.95, // Reasonable threshold - was too aggressive at 0.98
            "static" => 0.97,       // Reduced from 0.99 - was too aggressive
            "action" => 0.75,       // Lower threshold - keep more diverse frames
            _ => 0.85,
        };

        threshold
    }

    /// Extract JPEG frames from raw byte buffer by detecting JPEG boundaries
    fn extract_jpeg_frames(buffer: &[u8]) -> (Vec<Vec<u8>>, Vec<u8>) {
        let mut frames = Vec::new();
        let mut start = 0;
        let mut last_end = 0;

        // JPEG markers: SOI (0xFFD8) starts image, EOI (0xFFD9) ends image
        let mut i = 0;
        while i < buffer.len().saturating_sub(1) {
            if buffer[i] == 0xFF && buffer[i + 1] == 0xD9 {
                // End of Image marker
                let frame = buffer[start..=i + 1].to_vec();
                frames.push(frame);
                last_end = i + 2;

                // Look for next SOI marker
                let mut j = i + 2;
                while j < buffer.len().saturating_sub(1) {
                    if buffer[j] == 0xFF && buffer[j + 1] == 0xD8 {
                        // Start of Image marker
                        start = j;
                        i = j + 1;
                        break;
                    }
                    j += 1;
                }
                if j >= buffer.len().saturating_sub(1) {
                    break;
                }
            } else {
                i += 1;
            }
        }

        // Return complete frames and remaining buffer
        let remaining = if last_end < buffer.len() {
            buffer[last_end..].to_vec()
        } else {
            Vec::new()
        };

        (frames, remaining)
    }

    /// Process video frames directly in memory without temporary files
    pub async fn process_video_frames_in_memory<F>(
        &self,
        video_path: &str,
        frame_rate: f32,
        enable_scene_detection: bool,
        max_resolution: Option<u32>,
        mut callback: F,
        progress_callback: Option<Box<dyn Fn(VideoProcessingProgress) + Send + Sync>>,
    ) -> Result<usize>
    where
        F: FnMut(image::DynamicImage, VideoFrameMetadata) -> Result<()>,
    {
        let start_time = Instant::now();

        // Ensure FFmpeg is available
        let ffmpeg = match &self.ffmpeg_path {
            Some(path) => {
                app_log_info!("✅ Found FFmpeg at: {}", path.display());
                path
            }
            None => {
                app_log_error!("❌ FFmpeg not found in expected locations");
                return Err(anyhow!("Bundled FFmpeg not found, cannot process video"));
            }
        };

        // Check if the video file exists and is readable
        let video_file = Path::new(video_path);
        app_log_info!("🔍 Checking video file: {}", video_path);

        if !video_file.exists() {
            app_log_error!("❌ Video file not found: {}", video_path);
            return Err(anyhow!("Video file not found: {}", video_path));
        }

        let video_name = video_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        app_log_info!("🎬 Processing video in-memory: {}", video_name);

        // Send initial progress
        if let Some(ref progress_cb) = progress_callback {
            progress_cb(VideoProcessingProgress {
                phase: "metadata".to_string(),
                current_frame: 0,
                total_frames: 0,
                processed_frames: 0,
                phase_progress: 0.0,
                overall_progress: 0.0,
                current_operation: "Analyzing video metadata...".to_string(),
                fps: frame_rate,
                video_duration: 0.0,
                estimated_frames: 0,
                processing_speed: 0.0,
                time_remaining: 0.0,
            });
        }

        // Get video information first
        let video_info = self.get_video_metadata(video_path)?;

        app_log_info!(
            "📊 Video metadata: {}x{}, {:.1}s duration, {} codec",
            video_info.width,
            video_info.height,
            video_info.duration,
            video_info.codec
        );

        // Smart sampling analysis
        let video_analysis = self.analyze_video_characteristics(&video_info, video_path)?;
        let smart_fps = self.calculate_smart_sampling_rate(&video_analysis, frame_rate);

        app_log_info!(
            "🎯 Video Analysis: content_type='{}', motion_level={:.2}, smart_fps={}",
            video_analysis.content_type,
            video_analysis.motion_level,
            smart_fps
        );

        // Calculate estimated frames with smart sampling
        let estimated_frames = (video_info.duration * smart_fps as f64) as usize;

        // Send metadata complete progress
        if let Some(ref progress_cb) = progress_callback {
            progress_cb(VideoProcessingProgress {
                phase: "extraction".to_string(),
                current_frame: 0,
                total_frames: estimated_frames,
                processed_frames: 0,
                phase_progress: 0.0,
                overall_progress: 5.0,
                current_operation: format!(
                    "Starting in-memory frame extraction ({} frames expected)...",
                    estimated_frames
                ),
                fps: smart_fps,
                video_duration: video_info.duration,
                estimated_frames,
                processing_speed: 0.0,
                time_remaining: 0.0,
            });
        }

        // Build FFmpeg command for pipe output
        let mut vf_filters = vec![format!("fps={}", smart_fps)];

        // Add scaling filter if max_resolution is specified
        if let Some(max_res) = max_resolution {
            vf_filters.push(format!(
                "scale='min({},iw)':'min({},ih)':force_original_aspect_ratio=decrease",
                max_res, max_res
            ));
        }

        let vf_string = vf_filters.join(",");

        app_log_info!("🎬 Starting FFmpeg with in-memory processing");

        // Start FFmpeg with pipe output
        let mut cmd = Command::new(ffmpeg);
        cmd.arg("-i")
            .arg(video_path)
            .args([
                "-vf",
                &vf_string,
                "-c:v",
                "mjpeg",
                "-q:v",
                "3",
                "-f",
                "image2pipe", // Pipe output instead of files
                "pipe:1",     // Output to stdout
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        app_log_info!("🎬 Executing FFmpeg command: {:?}", cmd);

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn FFmpeg: {}", e))?;

        // Get stdout for reading frames
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get FFmpeg stdout"))?;

        // Initialize processing variables
        let mut frame_buffer = Vec::new();
        let mut frame_count = 0;
        let mut read_buffer = vec![0u8; 512 * 1024]; // 512KB read buffer
        let processing_start = Instant::now();

        // For scene change detection
        let mut last_frame_hash: Option<u64> = None;
        let similarity_threshold = self.get_similarity_threshold(&video_analysis);
        let mut skipped_frames = 0;

        app_log_info!("🔄 Starting in-memory frame processing loop");

        // Main processing loop
        loop {
            // Read chunk from FFmpeg stdout
            match stdout.read(&mut read_buffer) {
                Ok(0) => {
                    app_log_info!("📥 FFmpeg stdout EOF reached");
                    break; // EOF
                }
                Ok(n) => {
                    // Add new data to buffer
                    frame_buffer.extend_from_slice(&read_buffer[..n]);

                    // Extract complete JPEG frames
                    let (frames, remaining) = Self::extract_jpeg_frames(&frame_buffer);
                    frame_buffer = remaining; // Keep incomplete frame for next iteration

                    // Process each complete frame
                    for frame_bytes in frames {
                        // Load image from memory
                        let img = match image::load_from_memory(&frame_bytes) {
                            Ok(img) => img,
                            Err(e) => {
                                app_log_warn!("⚠️ Failed to load frame from memory: {}", e);
                                continue; // Skip corrupted frame, don't increment counter
                            }
                        };

                        // Create metadata with consistent frame numbering
                        let timestamp_seconds = frame_count as f64 / smart_fps as f64;
                        let metadata = VideoFrameMetadata {
                            video_path: video_path.to_string(),
                            timestamp: timestamp_seconds,
                            frame_number: frame_count,
                            video_duration: video_info.duration,
                            video_width: video_info.width,
                            video_height: video_info.height,
                        };

                        // Scene change detection
                        let mut frame_skipped = false;
                        if enable_scene_detection {
                            let current_hash = Self::calculate_frame_hash(&img);

                            if let Some(last_hash) = last_frame_hash {
                                let similarity =
                                    Self::calculate_hash_similarity(last_hash, current_hash);

                                if similarity > similarity_threshold {
                                    skipped_frames += 1;
                                    frame_skipped = true;
                                    app_log_debug!(
                                        "🔄 Skipping similar frame {} (similarity: {:.3})",
                                        frame_count,
                                        similarity
                                    );
                                } else {
                                    app_log_debug!(
                                        "✅ Keeping frame {} (similarity: {:.3})",
                                        frame_count,
                                        similarity
                                    );
                                }
                            }

                            if !frame_skipped {
                                last_frame_hash = Some(current_hash);
                            }
                        }

                        if !frame_skipped {
                            // Process the frame
                            if let Err(e) = callback(img, metadata) {
                                app_log_error!("❌ Failed to process frame {}: {}", frame_count, e);
                                continue; // Continue with next frame
                            }
                        }

                        frame_count += 1;

                        // Progress updates every 10 frames
                        if frame_count % 10 == 0 {
                            let elapsed = processing_start.elapsed().as_secs_f64();
                            let progress_percentage = if estimated_frames > 0 {
                                (frame_count as f64 / estimated_frames as f64) * 100.0
                            } else {
                                0.0
                            };

                            let processing_speed = if elapsed > 0.0 {
                                frame_count as f64 / elapsed
                            } else {
                                0.0
                            };
                            let remaining_frames = estimated_frames.saturating_sub(frame_count);
                            let estimated_remaining_time = if processing_speed > 0.0 {
                                remaining_frames as f64 / processing_speed
                            } else {
                                0.0
                            };

                            if let Some(ref progress_cb) = progress_callback {
                                progress_cb(VideoProcessingProgress {
                                    phase: "processing".to_string(),
                                    current_frame: frame_count,
                                    total_frames: estimated_frames,
                                    processed_frames: frame_count - skipped_frames,
                                    phase_progress: progress_percentage,
                                    overall_progress: 5.0 + (progress_percentage * 0.95), // 5% for metadata + 95% for processing
                                    current_operation: format!(
                                        "Processing frame {} (in-memory)",
                                        frame_count
                                    ),
                                    fps: smart_fps,
                                    video_duration: video_info.duration,
                                    estimated_frames,
                                    processing_speed,
                                    time_remaining: estimated_remaining_time,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    app_log_error!("❌ Error reading from FFmpeg stdout: {}", e);
                    break;
                }
            }
        }

        // Wait for FFmpeg to complete
        let exit_status = child
            .wait()
            .map_err(|e| anyhow!("Failed to wait for FFmpeg: {}", e))?;

        if !exit_status.success() {
            app_log_error!(
                "❌ FFmpeg process failed with exit code: {:?}",
                exit_status.code()
            );
            return Err(anyhow!("FFmpeg process failed"));
        }

        let total_time = start_time.elapsed();
        app_log_info!(
            "✅ In-memory video processing completed: {} frames ({} skipped) in {:.1}s",
            frame_count,
            skipped_frames,
            total_time.as_secs_f64()
        );

        // Send final progress
        if let Some(ref progress_cb) = progress_callback {
            progress_cb(VideoProcessingProgress {
                phase: "complete".to_string(),
                current_frame: frame_count,
                total_frames: frame_count,
                processed_frames: frame_count - skipped_frames,
                phase_progress: 100.0,
                overall_progress: 100.0,
                current_operation: format!(
                    "Completed: {} frames processed in-memory",
                    frame_count - skipped_frames
                ),
                fps: smart_fps,
                video_duration: video_info.duration,
                estimated_frames: frame_count,
                processing_speed: frame_count as f64 / total_time.as_secs_f64(),
                time_remaining: 0.0,
            });
        }

        Ok(frame_count - skipped_frames)
    }
}

/// Structure to hold video metadata
#[derive(Debug, Clone)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub duration: f64,
    pub codec: String,
}

#[derive(Debug, Clone)]
pub struct VideoAnalysis {
    pub content_type: String, // "talking_head", "action", "static", "mixed"
    pub motion_level: f32,    // 0.0 to 1.0
}
