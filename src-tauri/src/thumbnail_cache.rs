use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub static GENERATION_COUNT: AtomicUsize = AtomicUsize::new(0);
pub const CLEANUP_INTERVAL: usize = 50;
const MAX_CACHE_FILES: usize = 500;

// Try to get cached thumbnail without checking if original file exists
// This enables offline media preview support
pub async fn try_get_cached_thumbnail(
    file_path: &str,
    timestamp: f64,
    width: u32,
    height: u32,
) -> Result<Option<Vec<u8>>, String> {
    let cache_key = generate_cache_key(file_path, timestamp, width, height);
    let cache_path = get_cache_path(&cache_key)?;

    // Check if cached version exists
    if let Ok(cached_data) = fs::read(&cache_path) {
        return Ok(Some(cached_data));
    }

    Ok(None)
}

pub async fn get_cached_thumbnail(
    file_path: &str,
    timestamp: f64,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    // 1. Generate cache key
    let cache_key = generate_cache_key(file_path, timestamp, width, height);
    let cache_path = get_cache_path(&cache_key)?;

    // 2. Check if cached version exists and is valid
    if let Ok(cached_data) = fs::read(&cache_path) {
        if is_cache_valid(file_path, &cache_path)? {
            return Ok(cached_data);
        }
    }

    // 3. Generate new thumbnail
    let thumbnail_data = crate::ffmpeg_thumbnail::generate_ffmpeg_thumbnail(file_path, timestamp, width, height).await?;

    // 4. Save to cache
    if let Err(e) = fs::write(&cache_path, &thumbnail_data) {
        eprintln!("Warning: Failed to write thumbnail to cache: {}", e);
        // Continue anyway, return the generated thumbnail
    }

    // 5. Check if we should run cleanup
    let count = GENERATION_COUNT.fetch_add(1, Ordering::Relaxed);
    if count % CLEANUP_INTERVAL == 0 && count > 0 {
        // Run cleanup in background to avoid blocking
        tokio::spawn(async {
            println!("🧹 Running automatic cache cleanup (triggered after {} generations)", CLEANUP_INTERVAL);
            if let Err(e) = cleanup_old_cache(MAX_CACHE_FILES) {
                eprintln!("Automatic cache cleanup failed: {}", e);
            }
        });
    }

    Ok(thumbnail_data)
}

pub fn generate_cache_key(file_path: &str, timestamp: f64, width: u32, height: u32) -> String {
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    timestamp.to_bits().hash(&mut hasher);
    width.hash(&mut hasher);
    height.hash(&mut hasher);

    format!("thumb_{:016x}.jpg", hasher.finish())
}

pub fn get_cache_path(cache_key: &str) -> Result<PathBuf, String> {
    let cache_dir = get_thumbnail_cache_dir()?;
    Ok(cache_dir.join(cache_key))
}

pub fn is_cache_valid(original_file: &str, cache_file: &Path) -> Result<bool, String> {
    let original_metadata = fs::metadata(original_file)
        .map_err(|e| format!("Failed to get original file metadata: {}", e))?;
    let cache_metadata = fs::metadata(cache_file)
        .map_err(|e| format!("Failed to get cache file metadata: {}", e))?;

    let original_modified = original_metadata
        .modified()
        .map_err(|e| format!("Failed to get original file modified time: {}", e))?;
    let cache_modified = cache_metadata
        .modified()
        .map_err(|e| format!("Failed to get cache file modified time: {}", e))?;

    // Cache is valid if it's newer than the original file
    Ok(cache_modified >= original_modified)
}

pub fn get_thumbnail_cache_dir() -> Result<PathBuf, String> {
    // Use the same pattern as other app data - get app data directory
    let app_data = dirs::data_dir()
        .ok_or("Could not get app data directory")?
        .join("cosmos"); // Match your app identifier

    let cache_dir = app_data.join("thumbnails");

    // Create directory if it doesn't exist
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    Ok(cache_dir)
}

pub fn cleanup_old_cache(max_files: usize) -> Result<(), String> {
    let cache_dir = get_thumbnail_cache_dir()?;

    let mut entries: Vec<_> = fs::read_dir(cache_dir)
        .map_err(|e| format!("Failed to read cache directory: {}", e))?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let metadata = e.metadata().ok()?;
            let accessed = metadata.accessed().ok().or_else(|| metadata.modified().ok())?;
            Some((e.path(), accessed))
        })
        .collect();

    if entries.len() <= max_files {
        return Ok(());
    }

    // Sort by access time, oldest first
    entries.sort_by_key(|(_, accessed)| *accessed);

    // Remove oldest files beyond limit
    for (path, _) in entries.iter().take(entries.len() - max_files) {
        if let Err(e) = fs::remove_file(path) {
            eprintln!("Warning: Failed to remove old cache file {:?}: {}", path, e);
        }
    }

    Ok(())
}

