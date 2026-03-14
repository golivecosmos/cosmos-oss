use crate::app_log_debug;
use crate::app_log_warn;
use crate::utils::logger;
use crate::{app_log_error, app_log_info};
use tempfile;
use walkdir;
use zip;

/// Create an error report and save it to disk
#[tauri::command]
pub async fn create_error_report(
    error_type: String,
    error_message: String,
    stack_trace: Option<String>,
    user_description: Option<String>,
    reproduction_steps: Option<String>,
    app_state: Option<String>,
) -> Result<String, String> {
    app_log_info!(
        "🐛 ERROR REPORT: Creating error report for type: {}",
        error_type
    );

    let logger = logger::LOGGER.get_or_init(|| logger::AppLogger::new());

    let report = logger.create_error_report(
        error_type,
        error_message,
        stack_trace,
        user_description,
        reproduction_steps,
        app_state,
    );

    match logger.save_error_report(&report) {
        Ok(report_path) => {
            app_log_info!("✅ ERROR REPORT: Saved to {}", report_path.display());
            Ok(report.id)
        }
        Err(e) => {
            app_log_error!("❌ ERROR REPORT: Failed to save: {}", e);
            Err(format!("Failed to save error report: {}", e))
        }
    }
}

/// Get recent log entries
#[tauri::command]
pub async fn get_recent_logs(count: Option<usize>) -> Result<Vec<serde_json::Value>, String> {
    let count = count.unwrap_or(50);
    app_log_info!("📋 RECENT LOGS: Getting last {} log entries", count);

    let logger = logger::LOGGER.get_or_init(|| logger::AppLogger::new());
    let logs = logger.get_recent_logs(count);

    let json_logs: Vec<serde_json::Value> = logs
        .into_iter()
        .filter_map(|log| serde_json::to_value(&log).ok())
        .collect();

    app_log_info!("✅ RECENT LOGS: Retrieved {} log entries", json_logs.len());
    Ok(json_logs)
}

/// Package logs for support team
#[tauri::command]
pub async fn package_logs_for_support() -> Result<String, String> {
    app_log_info!("📦 PACKAGE LOGS: Creating support package");

    let logger = logger::LOGGER.get_or_init(|| logger::AppLogger::new());

    // Create a temporary directory for the package
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            app_log_error!("❌ PACKAGE LOGS: Failed to create temp dir: {}", e);
            return Err(format!("Failed to create temporary directory: {}", e));
        }
    };

    let package_dir = temp_dir.path().join("desktop_docs_logs");
    if let Err(e) = std::fs::create_dir_all(&package_dir) {
        app_log_error!("❌ PACKAGE LOGS: Failed to create package dir: {}", e);
        return Err(format!("Failed to create package directory: {}", e));
    }

    // Copy all log files
    let log_files = logger.get_all_log_files();
    let mut copied_files = 0;

    for log_file in log_files {
        if let Some(file_name) = log_file.file_name() {
            let dest_path = package_dir.join(file_name);
            if let Err(e) = std::fs::copy(&log_file, &dest_path) {
                app_log_warn!(
                    "⚠️ PACKAGE LOGS: Failed to copy {}: {}",
                    log_file.display(),
                    e
                );
            } else {
                copied_files += 1;
            }
        }
    }

    // Create system info file
    let system_info = logger.collect_system_info();
    let system_info_json = match serde_json::to_string_pretty(&system_info) {
        Ok(json) => json,
        Err(e) => {
            app_log_error!("❌ PACKAGE LOGS: Failed to serialize system info: {}", e);
            return Err(format!("Failed to serialize system info: {}", e));
        }
    };

    let system_info_path = package_dir.join("system_info.json");
    if let Err(e) = std::fs::write(&system_info_path, system_info_json) {
        app_log_warn!("⚠️ PACKAGE LOGS: Failed to write system info: {}", e);
    } else {
        copied_files += 1;
    }

    // Create a README for the support team
    let readme_content = format!(
        "Cosmos Support Package\n\
        ============================\n\n\
        Generated: {}\n\
        Session ID: {}\n\
        App Version: {}\n\n\
        Files included:\n\
        - app.log (and rotated versions): Main application logs\n\
        - errors.log (and rotated versions): Error-specific logs\n\
        - system_info.json: System and environment information\n\n\
        Instructions:\n\
        1. Review the logs for sensitive information before sharing\n\
        2. Attach this entire folder to your support request\n\
        3. Include a description of the issue you're experiencing\n",
        chrono::Utc::now().to_rfc3339(),
        logger.get_session_id(),
        env!("CARGO_PKG_VERSION")
    );

    let readme_path = package_dir.join("README.txt");
    if let Err(e) = std::fs::write(&readme_path, readme_content) {
        app_log_warn!("⚠️ PACKAGE LOGS: Failed to write README: {}", e);
    } else {
        copied_files += 1;
    }

    app_log_info!(
        "✅ PACKAGE LOGS: Created package with {} files",
        copied_files
    );

    // Create zip file in a persistent location instead of temp directory
    let app_data_dir = match crate::utils::path_utils::get_app_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            app_log_error!("❌ PACKAGE LOGS: Failed to get app data dir: {}", e);
            return Err(format!("Failed to get app data directory: {}", e));
        }
    };

    let logs_dir = app_data_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&logs_dir) {
        app_log_error!("❌ PACKAGE LOGS: Failed to create logs dir: {}", e);
        return Err(format!("Failed to create logs directory: {}", e));
    }

    // Create zip file with timestamp to avoid conflicts
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let zip_path = logs_dir.join(format!("desktop_docs_logs_{}.zip", timestamp));

    match create_zip_from_directory(&package_dir, &zip_path) {
        Ok(_) => {
            app_log_info!(
                "✅ PACKAGE LOGS: Created zip file at {}",
                zip_path.display()
            );

            // Verify the file exists before returning
            if zip_path.exists() {
                app_log_info!("✅ PACKAGE LOGS: Verified zip file exists");
                Ok(zip_path.to_string_lossy().to_string())
            } else {
                app_log_error!("❌ PACKAGE LOGS: Zip file was created but doesn't exist");
                Err("Zip file was created but cannot be found".to_string())
            }
        }
        Err(e) => {
            app_log_error!("❌ PACKAGE LOGS: Failed to create zip: {}", e);
            Err(format!("Failed to create zip file: {}", e))
        }
    }
}

/// Helper function to create a zip file from a directory
fn create_zip_from_directory(
    source_dir: &std::path::Path,
    zip_path: &std::path::Path,
) -> Result<(), String> {
    use zip::write::FileOptions;

    let file =
        std::fs::File::create(zip_path).map_err(|e| format!("Failed to create zip file: {}", e))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let walkdir = walkdir::WalkDir::new(source_dir);
    let it = walkdir.into_iter();

    for entry in it {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = path
            .strip_prefix(source_dir)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?;

        if path.is_file() {
            app_log_debug!("Adding file to zip: {}", name.display());
            zip.start_file(name.to_string_lossy(), options)
                .map_err(|e| format!("Failed to start zip file entry: {}", e))?;

            let mut f = std::fs::File::open(path)
                .map_err(|e| format!("Failed to open file for zipping: {}", e))?;

            std::io::copy(&mut f, &mut zip)
                .map_err(|e| format!("Failed to copy file to zip: {}", e))?;
        } else if !name.as_os_str().is_empty() {
            // Add directory entry
            app_log_debug!("Adding directory to zip: {}", name.display());
            zip.add_directory(name.to_string_lossy(), options)
                .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish zip file: {}", e))?;
    Ok(())
}
