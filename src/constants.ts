/// Supported image file extensions
///
/// This constant is used throughout the frontend to determine which file types
/// are supported for indexing, searching, and display. When adding new image formats,
/// update this constant and the corresponding backend constant in src-tauri/src/constants.rs
export const SUPPORTED_IMAGE_EXTENSIONS = [
  "jpg",
  "jpeg",
  "png",
  "gif",
  "webp",
  "bmp",
  "tiff",
  "tif",
] as const;

/// Supported video file extensions
export const SUPPORTED_VIDEO_EXTENSIONS = [
  "mp4",
  "mov",
  "avi",
  "webm",
  "mkv",
  "flv",
  "wmv",
  "m4v",
] as const;

/// Helper function to get all supported media extensions
///
/// This function combines the image and video extensions dynamically.
/// Use this when you need to ensure the arrays are always in sync.
export function getSupportedMediaExtensions(): readonly string[] {
  return [...SUPPORTED_IMAGE_EXTENSIONS, ...SUPPORTED_VIDEO_EXTENSIONS];
}

/// Check if a file extension is a supported image type
export function isSupportedImageExtension(ext: string): boolean {
  return SUPPORTED_IMAGE_EXTENSIONS.includes(ext.toLowerCase() as any);
}

/// Check if a file extension is a supported video type
export function isSupportedVideoExtension(ext: string): boolean {
  return SUPPORTED_VIDEO_EXTENSIONS.includes(ext.toLowerCase() as any);
}

/// Check if a file extension is a supported media type
export function isSupportedMediaExtension(ext: string): boolean {
  return getSupportedMediaExtensions().includes(ext.toLowerCase());
}
