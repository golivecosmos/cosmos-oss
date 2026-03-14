// use log::{info, error, debug, warn}; // Unused imports
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub module: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub thread: Option<String>,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorReport {
    pub id: String,
    pub timestamp: String,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub user_description: Option<String>,
    pub reproduction_steps: Option<String>,
    pub system_info: SystemInfo,
    pub recent_logs: Vec<LogEntry>,
    pub app_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub app_version: String,
    pub rust_version: String,
    pub memory_usage: Option<u64>,
    pub disk_space: Option<u64>,
    pub cpu_count: Option<usize>,
    pub uptime: Option<u64>,
}

pub struct AppLogger {
    log_file_path: PathBuf,
    error_log_path: PathBuf,
    session_id: String,
    max_log_size: u64,
    max_log_files: usize,
}

impl AppLogger {
    pub fn new() -> Self {
        // Get app data directory
        let app_dir = crate::utils::path_utils::get_app_data_dir()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Create logs directory
        let logs_dir = app_dir.join("logs");
        if !logs_dir.exists() {
            let _ = std::fs::create_dir_all(&logs_dir);
        }

        // Generate session ID
        let session_id = uuid::Uuid::new_v4().to_string();

        // Create log file paths
        let log_file_path = logs_dir.join("app.log");
        let error_log_path = logs_dir.join("errors.log");

        Self {
            log_file_path,
            error_log_path,
            session_id,
            max_log_size: 10 * 1024 * 1024, // 10MB
            max_log_files: 5,
        }
    }

    pub fn get_log_file_path(&self) -> &PathBuf {
        &self.log_file_path
    }

    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    /// Write a structured log entry
    pub fn write_log_entry(&self, entry: &LogEntry) -> Result<(), std::io::Error> {
        // Rotate logs if needed
        self.rotate_logs_if_needed()?;

        // Write to main log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)?;

        let log_line = format!(
            "[{}] [{}] {}\n",
            entry.timestamp, entry.level, entry.message
        );
        file.write_all(log_line.as_bytes())?;
        file.flush()?;

        // Also write errors to separate error log
        if entry.level == "ERROR" {
            let mut error_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.error_log_path)?;

            let error_entry = serde_json::to_string(entry).unwrap_or_else(|_| log_line.clone());
            error_file.write_all(format!("{}\n", error_entry).as_bytes())?;
            error_file.flush()?;
        }

        Ok(())
    }

    /// Get recent log entries for error reporting
    pub fn get_recent_logs(&self, count: usize) -> Vec<LogEntry> {
        use std::io::{BufRead, BufReader};

        let mut logs = Vec::new();

        if let Ok(file) = File::open(&self.log_file_path) {
            let reader = BufReader::new(file);
            let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

            // Take the last `count` lines
            for line in lines.iter().rev().take(count).rev() {
                if let Ok(entry) = self.parse_log_line(line) {
                    logs.push(entry);
                }
            }
        }

        logs
    }

    /// Parse a log line back into a LogEntry
    fn parse_log_line(&self, line: &str) -> Result<LogEntry, serde_json::Error> {
        // Try to parse as JSON first (for structured logs)
        if let Ok(entry) = serde_json::from_str::<LogEntry>(line) {
            return Ok(entry);
        }

        // Fallback: parse simple format [timestamp] [level] message
        let parts: Vec<&str> = line.splitn(3, "] ").collect();
        if parts.len() >= 3 {
            let timestamp = parts[0].trim_start_matches('[');
            let level = parts[1].trim_start_matches('[');
            let message = parts[2];

            Ok(LogEntry {
                timestamp: timestamp.to_string(),
                level: level.to_string(),
                message: message.to_string(),
                module: None,
                file: None,
                line: None,
                thread: None,
                session_id: self.session_id.clone(),
            })
        } else {
            // Create a basic entry for unparseable lines
            Ok(LogEntry {
                timestamp: Utc::now().to_rfc3339(),
                level: "INFO".to_string(),
                message: line.to_string(),
                module: None,
                file: None,
                line: None,
                thread: None,
                session_id: self.session_id.clone(),
            })
        }
    }

    /// Rotate logs if they exceed size limit
    fn rotate_logs_if_needed(&self) -> Result<(), std::io::Error> {
        if let Ok(metadata) = std::fs::metadata(&self.log_file_path) {
            if metadata.len() > self.max_log_size {
                self.rotate_log_file(&self.log_file_path)?;
            }
        }

        if let Ok(metadata) = std::fs::metadata(&self.error_log_path) {
            if metadata.len() > self.max_log_size {
                self.rotate_log_file(&self.error_log_path)?;
            }
        }

        Ok(())
    }

    /// Rotate a specific log file
    fn rotate_log_file(&self, log_path: &PathBuf) -> Result<(), std::io::Error> {
        let log_dir = log_path.parent().unwrap();
        let log_name = log_path.file_stem().unwrap().to_str().unwrap();
        let log_ext = log_path.extension().unwrap_or_default().to_str().unwrap();

        // Shift existing rotated files
        for i in (1..self.max_log_files).rev() {
            let old_path = log_dir.join(format!("{}.{}.{}", log_name, i, log_ext));
            let new_path = log_dir.join(format!("{}.{}.{}", log_name, i + 1, log_ext));

            if old_path.exists() {
                if i + 1 >= self.max_log_files {
                    // Delete the oldest file
                    let _ = std::fs::remove_file(&old_path);
                } else {
                    let _ = std::fs::rename(&old_path, &new_path);
                }
            }
        }

        // Move current log to .1
        let rotated_path = log_dir.join(format!("{}.1.{}", log_name, log_ext));
        let _ = std::fs::rename(log_path, &rotated_path);

        Ok(())
    }

    /// Collect system information for error reports
    pub fn collect_system_info(&self) -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_string(),
            os_version: Self::get_os_version(),
            arch: std::env::consts::ARCH.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            rust_version: Self::get_rust_version(),
            memory_usage: Self::get_memory_usage(),
            disk_space: Self::get_disk_space(),
            cpu_count: Some(num_cpus::get()),
            uptime: Self::get_uptime(),
        }
    }

    fn get_os_version() -> String {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("sw_vers").arg("-productVersion").output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return version.trim().to_string();
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("cmd").args(&["/C", "ver"]).output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    return version.trim().to_string();
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
                for line in contents.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return line
                            .split('=')
                            .nth(1)
                            .unwrap_or("Unknown")
                            .trim_matches('"')
                            .to_string();
                    }
                }
            }
        }

        "Unknown".to_string()
    }

    fn get_rust_version() -> String {
        std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string())
    }

    fn get_memory_usage() -> Option<u64> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("ps")
                .args(&["-o", "rss=", "-p"])
                .arg(std::process::id().to_string())
                .output()
            {
                if let Ok(rss_str) = String::from_utf8(output.stdout) {
                    if let Ok(rss_kb) = rss_str.trim().parse::<u64>() {
                        return Some(rss_kb * 1024); // Convert KB to bytes
                    }
                }
            }
        }
        None
    }

    fn get_disk_space() -> Option<u64> {
        if let Ok(app_dir) = crate::utils::path_utils::get_app_data_dir() {
            #[cfg(unix)]
            {
                use std::ffi::CString;
                use std::mem;

                if let Ok(path_cstr) = CString::new(app_dir.to_string_lossy().as_bytes()) {
                    unsafe {
                        let mut statvfs: libc::statvfs = mem::zeroed();
                        if libc::statvfs(path_cstr.as_ptr(), &mut statvfs) == 0 {
                            let free_bytes = (statvfs.f_bavail as u64) * (statvfs.f_frsize as u64);
                            return Some(free_bytes);
                        }
                    }
                }
            }
        }
        None
    }

    fn get_uptime() -> Option<u64> {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("uptime").output() {
                if let Ok(uptime_str) = String::from_utf8(output.stdout) {
                    // Parse uptime from output (this is a simplified version)
                    if let Some(days_pos) = uptime_str.find(" days") {
                        if let Some(up_pos) = uptime_str.find("up ") {
                            let days_str = &uptime_str[up_pos + 3..days_pos];
                            if let Ok(days) = days_str.trim().parse::<u64>() {
                                return Some(days * 24 * 3600); // Convert to seconds
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Create a comprehensive error report
    pub fn create_error_report(
        &self,
        error_type: String,
        error_message: String,
        stack_trace: Option<String>,
        user_description: Option<String>,
        reproduction_steps: Option<String>,
        app_state: Option<String>,
    ) -> ErrorReport {
        ErrorReport {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            error_type,
            error_message,
            stack_trace,
            user_description,
            reproduction_steps,
            system_info: self.collect_system_info(),
            recent_logs: self.get_recent_logs(100), // Last 100 log entries
            app_state,
        }
    }

    /// Save error report to file
    pub fn save_error_report(&self, report: &ErrorReport) -> Result<PathBuf, std::io::Error> {
        let reports_dir = self.log_file_path.parent().unwrap().join("error_reports");
        if !reports_dir.exists() {
            std::fs::create_dir_all(&reports_dir)?;
        }

        let report_file = reports_dir.join(format!("error_report_{}.json", report.id));
        let report_json = serde_json::to_string_pretty(report)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        std::fs::write(&report_file, report_json)?;
        Ok(report_file)
    }

    /// Get all log files for packaging
    pub fn get_all_log_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let log_dir = self.log_file_path.parent().unwrap();

        if let Ok(entries) = std::fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("app.log") || name.starts_with("errors.log") {
                            files.push(path);
                        }
                    }
                }
            }
        }

        files
    }
}

// Global logger instance
pub static LOGGER: OnceLock<AppLogger> = OnceLock::new();

// Enhanced logging macros that create structured log entries
#[macro_export]
macro_rules! app_log_info {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            log::info!("{}", message);

            // Only print to console in debug builds or for important messages
            if cfg!(debug_assertions) || message.contains("ERROR") || message.contains("WARN") || message.contains("STARTUP") {
                println!("ℹ️  INFO: {}", message);
            }

            if let Some(logger) = crate::utils::logger::LOGGER.get() {
                let entry = crate::utils::logger::LogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "INFO".to_string(),
                    message: message.clone(),
                    module: Some(module_path!().to_string()),
                    file: Some(file!().to_string()),
                    line: Some(line!()),
                    thread: Some(format!("{:?}", std::thread::current().id())),
                    session_id: logger.get_session_id().to_string(),
                };
                let _ = logger.write_log_entry(&entry);
            }
        }
    };
}

#[macro_export]
macro_rules! app_log_debug {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            log::debug!("{}", message);

            // Only print debug messages in debug builds
            if cfg!(debug_assertions) {
                println!("🔍 DEBUG: {}", message);
            }

            // Only write debug logs to file in debug builds
            if cfg!(debug_assertions) {
                if let Some(logger) = crate::utils::logger::LOGGER.get() {
                    let entry = crate::utils::logger::LogEntry {
                        timestamp: chrono::Utc::now().to_rfc3339(),
                        level: "DEBUG".to_string(),
                        message: message.clone(),
                        module: Some(module_path!().to_string()),
                        file: Some(file!().to_string()),
                        line: Some(line!()),
                        thread: Some(format!("{:?}", std::thread::current().id())),
                        session_id: logger.get_session_id().to_string(),
                    };
                    let _ = logger.write_log_entry(&entry);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! app_log_warn {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            log::warn!("{}", message);

            // Always print warnings to console
            println!("⚠️  WARN: {}", message);

            if let Some(logger) = crate::utils::logger::LOGGER.get() {
                let entry = crate::utils::logger::LogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "WARN".to_string(),
                    message: message.clone(),
                    module: Some(module_path!().to_string()),
                    file: Some(file!().to_string()),
                    line: Some(line!()),
                    thread: Some(format!("{:?}", std::thread::current().id())),
                    session_id: logger.get_session_id().to_string(),
                };
                let _ = logger.write_log_entry(&entry);
            }
        }
    };
}

#[macro_export]
macro_rules! app_log_error {
    ($($arg:tt)*) => {
        {
            let message = format!($($arg)*);
            log::error!("{}", message);

            // Always print errors to console
            println!("❌ ERROR: {}", message);

            if let Some(logger) = crate::utils::logger::LOGGER.get() {
                let entry = crate::utils::logger::LogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "ERROR".to_string(),
                    message: message.clone(),
                    module: Some(module_path!().to_string()),
                    file: Some(file!().to_string()),
                    line: Some(line!()),
                    thread: Some(format!("{:?}", std::thread::current().id())),
                    session_id: logger.get_session_id().to_string(),
                };
                let _ = logger.write_log_entry(&entry);
            }
        }
    };
}
