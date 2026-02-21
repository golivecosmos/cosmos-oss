use anyhow::{Result, Context};
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use base64::{Engine, engine::general_purpose::STANDARD};
use crate::models::file_item::{FileItem, FileMetadata};

/// Service for handling file system operations
pub struct FileService;

impl FileService {
    /// Create a new file service
    pub fn new() -> Self {
        Self
    }
    
    /// List contents of a directory
    pub fn list_directory(&self, path: &str) -> Result<Vec<FileItem>> {
        let entries = fs::read_dir(path).context(format!("Failed to read directory: {}", path))?;
        
        let mut items = Vec::new();
        
        for entry in entries {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                
                // Skip hidden files, .DS_Store, and Thumbs.db
                if name.starts_with(".") || 
                   name == "DS_Store" || 
                   name == ".DS_Store" || 
                   name == "Thumbs.db" {
                    continue;
                }
                
                let path_buf = entry.path();
                let is_dir = path_buf.is_dir();
                let path_str = path_buf.to_string_lossy().to_string();
                
                // Get child count for directories
                let child_count = if is_dir {
                    match fs::read_dir(&path_buf) {
                        Ok(children) => {
                            Some(children
                                .filter(|e| {
                                    if let Ok(e) = e {
                                        let name = e.file_name().to_string_lossy().to_string();
                                        !(name.starts_with(".") || 
                                          name == "DS_Store" || 
                                          name == ".DS_Store" || 
                                          name == "Thumbs.db")
                                    } else {
                                        false
                                    }
                                })
                                .count())
                        },
                        Err(_) => Some(0)
                    }
                } else {
                    None
                };
                
                items.push(FileItem {
                    name,
                    path: path_str,
                    is_dir,
                    child_count,
                });
            }
        }
        
        // Sort directories first, then files
        items.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(items)
    }
    
    /// List contents of a directory recursively
    pub fn list_directory_recursive(&self, path: &str) -> Result<Vec<FileItem>> {
        let mut result = Vec::new();
        
        // Skip hidden directories
        if Path::new(path).file_name()
            .map(|name| name.to_string_lossy().starts_with("."))
            .unwrap_or(false) 
        {
            return Ok(result);
        }
        
        // Use walkdir to recursively walk the directory
        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok()) 
        {
            let name = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden files, .DS_Store, and Thumbs.db
            if name.starts_with(".") || 
               name == "DS_Store" || 
               name == ".DS_Store" || 
               name == "Thumbs.db" {
                continue;
            }
            
            let path_buf = entry.path();
            let is_dir = path_buf.is_dir();
            let path_str = path_buf.to_string_lossy().to_string();
            
            // Get child count for directories (only immediate children, not recursively)
            let child_count = if is_dir {
                match fs::read_dir(&path_buf) {
                    Ok(children) => {
                        Some(children
                            .filter(|e| {
                                if let Ok(e) = e {
                                    let name = e.file_name().to_string_lossy().to_string();
                                    !(name.starts_with(".") || 
                                      name == "DS_Store" || 
                                      name == ".DS_Store" || 
                                      name == "Thumbs.db")
                                } else {
                                    false
                                }
                            })
                            .count())
                    },
                    Err(_) => Some(0)
                }
            } else {
                None
            };
            
            result.push(FileItem {
                name,
                path: path_str,
                is_dir,
                child_count,
            });
        }
        
        // Sort directories first, then files
        result.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        
        Ok(result)
    }
    
    /// Get file metadata
    pub fn get_file_metadata(&self, path: &str) -> Result<FileMetadata> {
        let metadata = fs::metadata(path).context(format!("Failed to get metadata for: {}", path))?;
        
        let created = metadata.created()
            .context(format!("Failed to get creation time for: {}", path))?;
        let modified = metadata.modified()
            .context(format!("Failed to get modification time for: {}", path))?;
        
        let created_datetime: DateTime<Utc> = created.into();
        let modified_datetime: DateTime<Utc> = modified.into();
        
        Ok(FileMetadata {
            name: Path::new(path).file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string()),
            path: path.to_string(),
            size: metadata.len(),
            created_at: created_datetime.to_rfc3339(),
            modified_at: modified_datetime.to_rfc3339(),
            is_dir: metadata.is_dir(),
        })
    }
    
    /// Read file content as text or base64
    pub fn read_file_content(&self, path: &str) -> Result<String> {
        // Check file extension against known binary types
        let lower_path = path.to_lowercase();
        let is_binary = lower_path.ends_with(".jpg") 
            || lower_path.ends_with(".jpeg")
            || lower_path.ends_with(".png")
            || lower_path.ends_with(".gif")
            || lower_path.ends_with(".tiff")
            || lower_path.ends_with(".tif")
            || lower_path.ends_with(".webp")
            || lower_path.ends_with(".bmp")
            || lower_path.ends_with(".pdf")
            || lower_path.ends_with(".mp4")
            || lower_path.ends_with(".mov")
            || lower_path.ends_with(".avi")
            || lower_path.ends_with(".webm");

        let mut file = fs::File::open(path).context(format!("Failed to open file: {}", path))?;
        
        if is_binary {
            let mut buffer = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buffer)
                .context(format!("Failed to read binary file: {}", path))?;
            Ok(STANDARD.encode(&buffer))
        } else {
            let mut contents = String::new();
            std::io::Read::read_to_string(&mut file, &mut contents)
                .context(format!("Failed to read text file: {}", path))?;
            Ok(contents)
        }
    }
    
    /// Check if a path is a directory
    pub fn is_directory(&self, path: &str) -> bool {
        Path::new(path).is_dir()
    }
} 