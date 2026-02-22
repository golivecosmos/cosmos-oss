// FFmpeg-only thumbnail generation to avoid Objective-C FFI issues
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;

// FFmpeg-only implementation that avoids Objective-C FFI completely
pub async fn generate_ffmpeg_thumbnail(file_path: &str, timestamp: f64, width: u32, height: u32) -> Result<Vec<u8>, String> {
    // First verify the file exists
    if !Path::new(file_path).exists() {
        return Err(format!("Video file not found: {}", file_path));
    }
    
    // Get the path to the bundled ffmpeg
    let ffmpeg_path = get_bundled_ffmpeg_path();
    
    if !ffmpeg_path.exists() {
        return Err(format!("Bundled ffmpeg not found at: {}", ffmpeg_path.display()));
    }
    
    // Create a temporary file for the output
    let temp_file = NamedTempFile::new()
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let output_path = temp_file.path().to_str()
        .ok_or_else(|| "Failed to get temp file path".to_string())?;
    let jpeg_path = format!("{}.jpg", output_path);
    
    // Use bundled ffmpeg to generate thumbnail with hardware acceleration
    
    // Create owned strings to avoid borrowing issues
    let timestamp_str = timestamp.to_string();
    
    let output = Command::new(&ffmpeg_path)
        .arg("-hwaccel").arg("auto")
        .arg("-ss").arg(&timestamp_str)
        .arg("-i").arg(file_path)
        .arg("-vframes").arg("1")
        .arg("-vf").arg(&format!("scale={}:{}:flags=lanczos:force_original_aspect_ratio=decrease:eval=frame", width, height))
        .arg("-q:v").arg("2")
        .arg("-f").arg("image2")
        .arg(&jpeg_path)
        .arg("-y")
        .arg("-loglevel").arg("error") // Reduce log noise
        .output()
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;
    
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {}", stderr));
    }
    
    // Read the generated thumbnail
    let jpeg_data = std::fs::read(&jpeg_path)
        .map_err(|e| format!("Failed to read thumbnail: {}", e))?;
    
    // Clean up
    let _ = std::fs::remove_file(&jpeg_path);
    
    Ok(jpeg_data)
}

pub fn get_bundled_ffmpeg_path() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin/ffmpeg")
    }

    #[cfg(not(debug_assertions))]
    {
        match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|dir| dir.join("../Resources/bin/ffmpeg"))
                .unwrap_or_default(),
            Err(_) => PathBuf::new(),
        }
    }
}
