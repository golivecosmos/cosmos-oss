import { invoke } from '@tauri-apps/api/core';

interface ThumbnailOptions {
  timestamp?: number;
  width?: number;
  height?: number;
}

interface ThumbnailResult {
  success: boolean;
  data?: string;
  error?: string;
  width?: number;
  height?: number;
}

export async function generateThumbnail(
  filePath: string,
  options: ThumbnailOptions = {}
): Promise<string> {
  const timestamp = options.timestamp || 1.0;
  const width = options.width || 480;
  const height = options.height || 270;
  
  // Convert asset:// URLs to file system paths
  let actualFilePath = filePath;
  if (filePath.startsWith('asset://localhost/')) {
    actualFilePath = decodeURIComponent(filePath.replace('asset://localhost/', ''));
  }
  
  
  try {
    const result: ThumbnailResult = await invoke('generate_video_thumbnail', {
      filePath: actualFilePath,
      timestampSeconds: timestamp,
      width,
      height,
    });

    if (result.success && result.data) {
      return `data:image/jpeg;base64,${result.data}`;
    } else {
      throw new Error(result.error || 'Failed to generate thumbnail');
    }
  } catch (error) {
    console.error('Thumbnail generation failed:', error);
    throw error;
  }
}

// Optional: Add simple memory cache for recently generated thumbnails
const thumbnailCache = new Map<string, string>();
const MAX_CACHE_SIZE = 100;

export async function getCachedThumbnail(
  filePath: string,
  options: ThumbnailOptions = {}
): Promise<string> {
  // Convert asset:// URLs to file system paths for cache key consistency
  let actualFilePath = filePath;
  if (filePath.startsWith('asset://localhost/')) {
    actualFilePath = decodeURIComponent(filePath.replace('asset://localhost/', ''));
  }
  
  const cacheKey = `${actualFilePath}@${options.timestamp || 0}@${options.width || 480}x${options.height || 270}`;

  if (thumbnailCache.has(cacheKey)) {
    return thumbnailCache.get(cacheKey)!;
  }

  const thumbnail = await generateThumbnail(filePath, options);

  // Simple LRU eviction
  if (thumbnailCache.size >= MAX_CACHE_SIZE) {
    const firstKey = thumbnailCache.keys().next().value;
    thumbnailCache.delete(firstKey);
  }

  thumbnailCache.set(cacheKey, thumbnail);
  return thumbnail;
}