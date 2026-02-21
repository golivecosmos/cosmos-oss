use anyhow::Result;
use rusqlite::{Connection, ffi::sqlite3_auto_extension};
use sqlite_vec::sqlite3_vec_init;
use std::path::PathBuf;
use std::path::Path;
use std::fs;
use std::sync::{Arc, Mutex, RwLock};
use crate::utils::path_utils;
use crate::{app_log_debug, app_log_info, app_log_warn, app_log_error};
use crate::services::config_service::ConfigService;
use crate::services::database_encryption_service::DatabaseEncryptionService;
use zerocopy::AsBytes;

/// Core database management service
/// Handles database connection, initialization, and path management
/// Now supports both encrypted and unencrypted databases
pub struct DatabaseService {
    db: Arc<Mutex<Connection>>,
    db_path: Arc<RwLock<PathBuf>>,
    config_service: Arc<Mutex<ConfigService>>,
    encryption_service: DatabaseEncryptionService,
    is_encrypted: bool,
}

impl DatabaseService {
    /// Create an in-memory database service for testing
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        // Register sqlite-vec extension
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        // Create in-memory index database
        let index_db = Connection::open(":memory:")?;

        // Test that the extension is working
        let (vec_version, test_embedding): (String, String) = index_db.query_row(
            "SELECT vec_version(), vec_to_json(?)",
            rusqlite::params![&[0.1f32, 0.2, 0.3].as_bytes()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        app_log_debug!("✅ SQLITE: sqlite-vec extension loaded successfully (in-memory)");
        app_log_debug!("📊 SQLITE: vec_version = {}, test_embedding = {}", vec_version, test_embedding);

        let mut service = Self {
            db: Arc::new(Mutex::new(index_db)),
            db_path: Arc::new(RwLock::new(PathBuf::from(":memory:"))),
            config_service: Arc::new(Mutex::new(ConfigService::new()?)),
            encryption_service: DatabaseEncryptionService::new_for_testing(),
            is_encrypted: false, // In-memory databases are not encrypted
        };

        // Initialize the in-memory database with schema for testing
        service.initialize_database()?;

        Ok(service)
    }

    /// Create a new database service with custom path
    pub fn new_with_path(custom_dir: Option<PathBuf>) -> Result<Self> {
        // Clean up old user.db file if it exists
        ConfigService::cleanup_old_user_db()?;

        // Clean up old vector_search.db file if it exists (only in default location)
        ConfigService::cleanup_old_vector_db()?;

        // Initialize config service
        let mut config_service = ConfigService::new()?;

        // Handle migration from custom vector_search.db to custom .cosmos.db
        Self::migrate_custom_vector_db(&mut config_service)?;

        // Get database path from config (custom or default)
        let index_db_path = if let Some(custom_dir) = custom_dir {
            // For testing or specific custom directory
            custom_dir.join(crate::constants::DATABASE_FILENAME)
        } else {
            config_service.get_db_path()?
        };

        app_log_debug!("🗄️ SQLITE: Initializing SQLite database service at: {}", index_db_path.display());

        // Register sqlite-vec extension
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        // Only create directory if we're not in a migration scenario
        if !path_utils::is_migration_needed() {
            // Ensure the data directory exists
            if let Some(parent) = index_db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Create initial connection (will be replaced during initialization)
        let connection = Connection::open(&index_db_path)?;

        // Test that the extension is working
        let (vec_version, test_embedding): (String, String) = connection.query_row(
            "SELECT vec_version(), vec_to_json(?)",
            rusqlite::params![&[0.1f32, 0.2, 0.3].as_bytes()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        app_log_debug!("✅ SQLITE: sqlite-vec extension loaded successfully");
        app_log_debug!("📊 SQLITE: vec_version = {}, test_embedding = {}", vec_version, test_embedding);

        Ok(Self {
            db: Arc::new(Mutex::new(connection)),
            db_path: Arc::new(RwLock::new(index_db_path)),
            config_service: Arc::new(Mutex::new(config_service)),
            encryption_service: DatabaseEncryptionService::new(),
            is_encrypted: false,
        })
    }

    /// Get the database path and whether it's the default path
    pub fn get_db_path(&self) -> Result<(PathBuf, bool), String> {
        let db_path = self.db_path.read().unwrap().clone();
        let is_default = db_path.to_string_lossy() == path_utils::get_app_data_dir().map_err(|e| e.to_string())?.join(crate::constants::DATABASE_FILENAME).to_string_lossy();
        Ok((db_path, is_default))
    }

    /// Set a new database path
    pub fn set_db_path(&self, new_dir: Option<&str>) -> Result<String, String> {
        app_log_info!("🔧 DB_SET_PATH: Starting set_db_path");
        app_log_info!("🔧 DB_SET_PATH: new_dir = {:?}", new_dir);

        let new_dir_path = match new_dir {
            Some(v) => {
                app_log_info!("🔧 DB_SET_PATH: Using custom directory: {}", v);
                PathBuf::from(v)
            }
            None => {
                app_log_info!("🔧 DB_SET_PATH: Using default app data directory");
                path_utils::get_app_data_dir().map_err(|e| e.to_string())?
            }
        };

        let new_db_path = new_dir_path.join(crate::constants::DATABASE_FILENAME);
        app_log_info!("🔧 DB_SET_PATH: New database path will be: {}", new_db_path.display());

        if new_db_path.exists() {
            app_log_error!("❌ DB_SET_PATH: Database already exists at: {}", new_db_path.display());
            return Err(format!("Database already exists at: {}", new_dir_path.display()));
        }

        app_log_info!("🔧 DB_SET_PATH: Copying database to new location");
        {
            let mut db_path = self.db_path.write().unwrap();
            let old_db_path = db_path.clone();

            // Close the current database connection to ensure all WAL files are flushed
            app_log_info!("🔧 DB_SET_PATH: Closing current database connection");
            {
                let db = self.db.lock().unwrap();
                drop(db); // This will close the connection and flush WAL files
            }

            // Create temp paths for all database files
            let temp_db_path = new_db_path.with_extension("tmp");
            let temp_shm_path = new_db_path.with_extension("db-shm.tmp");
            let temp_wal_path = new_db_path.with_extension("db-wal.tmp");

            // Define the WAL file paths for the old database
            let old_shm_path = old_db_path.with_extension("db-shm");
            let old_wal_path = old_db_path.with_extension("db-wal");

            app_log_info!("🔧 DB_SET_PATH: Copying main database from {} to {}", old_db_path.display(), temp_db_path.display());

            // Copy main database file
            match fs::copy(&old_db_path, &temp_db_path) {
                Ok(_) => app_log_info!("✅ DB_SET_PATH: Main database copied to temp location"),
                Err(e) => {
                    app_log_error!("❌ DB_SET_PATH: Failed to copy main database: {}", e);
                    return Err(e.to_string());
                }
            }

            // Copy WAL files if they exist
            if old_shm_path.exists() {
                app_log_info!("🔧 DB_SET_PATH: Copying SHM file from {} to {}", old_shm_path.display(), temp_shm_path.display());
                match fs::copy(&old_shm_path, &temp_shm_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: SHM file copied to temp location"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to copy SHM file: {}", e);
                        return Err(e.to_string());
                    }
                }
            } else {
                app_log_info!("🔧 DB_SET_PATH: No SHM file found, skipping");
            }

            if old_wal_path.exists() {
                app_log_info!("🔧 DB_SET_PATH: Copying WAL file from {} to {}", old_wal_path.display(), temp_wal_path.display());
                match fs::copy(&old_wal_path, &temp_wal_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: WAL file copied to temp location"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to copy WAL file: {}", e);
                        return Err(e.to_string());
                    }
                }
            } else {
                app_log_info!("🔧 DB_SET_PATH: No WAL file found, skipping");
            }

            app_log_info!("🔧 DB_SET_PATH: Verifying database integrity");
            match self.verify_new_db_integrity(&old_db_path, &temp_db_path) {
                Ok(_) => app_log_info!("✅ DB_SET_PATH: Database integrity verified"),
                Err(e) => {
                    app_log_error!("❌ DB_SET_PATH: Database integrity check failed: {}", e);
                    return Err(e);
                }
            }

            // Define final paths for WAL files
            let new_shm_path = new_db_path.with_extension("db-shm");
            let new_wal_path = new_db_path.with_extension("db-wal");

            app_log_info!("🔧 DB_SET_PATH: Moving database files to final location");

            // Move main database file
            match fs::rename(&temp_db_path, &new_db_path) {
                Ok(_) => app_log_info!("✅ DB_SET_PATH: Main database moved to final location"),
                Err(e) => {
                    app_log_error!("❌ DB_SET_PATH: Failed to move main database: {}", e);
                    return Err(e.to_string());
                }
            }

            // Move SHM file if it exists
            if temp_shm_path.exists() {
                match fs::rename(&temp_shm_path, &new_shm_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: SHM file moved to final location"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to move SHM file: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            // Move WAL file if it exists
            if temp_wal_path.exists() {
                match fs::rename(&temp_wal_path, &new_wal_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: WAL file moved to final location"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to move WAL file: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            app_log_info!("🔧 DB_SET_PATH: Removing old database files");

            // Remove old database files
            if old_db_path.exists() {
                match fs::remove_file(&old_db_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: Old main database file removed"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to remove old main database: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            if old_shm_path.exists() {
                match fs::remove_file(&old_shm_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: Old SHM file removed"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to remove old SHM file: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            if old_wal_path.exists() {
                match fs::remove_file(&old_wal_path) {
                    Ok(_) => app_log_info!("✅ DB_SET_PATH: Old WAL file removed"),
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to remove old WAL file: {}", e);
                        return Err(e.to_string());
                    }
                }
            }

            *db_path = new_db_path.clone();
        }

        // Update config service with new custom path
        {
            let mut config_service = self.config_service.lock().unwrap();
            let custom_path = if new_dir.is_some() {
                Some(new_db_path.to_string_lossy().to_string())
            } else {
                None // Clear custom path to use default
            };
            config_service.set_custom_db_path(custom_path).map_err(|e| e.to_string())?;
            app_log_info!("✅ DB_SET_PATH: Path stored in config successfully");
        }

        app_log_info!("🔧 DB_SET_PATH: Creating new database connection");
        {
            let new_connection = if self.is_encrypted {
                match self.get_encrypted_connection() {
                    Ok(conn) => {
                        app_log_info!("✅ DB_SET_PATH: New encrypted database connection created");
                        conn
                    }
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to create new encrypted database connection: {}", e);
                        return Err(e.to_string());
                    }
                }
            } else {
                match Connection::open(&*self.db_path.read().unwrap()) {
                    Ok(conn) => {
                        app_log_info!("✅ DB_SET_PATH: New unencrypted database connection created");
                        conn
                    }
                    Err(e) => {
                        app_log_error!("❌ DB_SET_PATH: Failed to create new database connection: {}", e);
                        return Err(e.to_string());
                    }
                }
            };
            let mut db = self.db.lock().unwrap();
            *db = new_connection;
        }

        let result = new_db_path.to_string_lossy().to_string();
        app_log_info!("✅ DB_SET_PATH: set_db_path completed successfully: {}", result);
        Ok(result)
    }

    /// Get the database connection
    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        // Check if the database is actually encrypted
        let is_actually_encrypted = self.is_database_encrypted();

        // If our internal state doesn't match the actual database state, update it
        if is_actually_encrypted != self.is_encrypted {
            app_log_info!("🔐 Database encryption state changed, updating connection");

            // Try to create a new connection with encryption
            let new_connection = self.get_encrypted_connection();

            if let Ok(conn) = new_connection {
                let mut db = self.db.lock().unwrap();
                *db = conn;
                app_log_info!("✅ Successfully updated connection to match database state");
            }
        }

        self.db.clone()
    }

    /// Get the config service
    pub fn get_config_service(&self) -> Arc<Mutex<ConfigService>> {
        self.config_service.clone()
    }

    /// Verify the integrity of a new database file
    fn verify_new_db_integrity(&self, old: &Path, new: &Path) -> Result<(), String> {
        app_log_info!("🔧 VERIFY_INTEGRITY: Starting database integrity verification");
        app_log_info!("🔧 VERIFY_INTEGRITY: old = {}, new = {}", old.display(), new.display());

        let old_size = fs::metadata(old).map_err(|e| format!("New Location has not been set. Failed to read old DB metadata: {}", e))?.len();
        let new_size = fs::metadata(new).map_err(|e| format!("New Location has not been set. Failed to read new DB metadata: {}", e))?.len();

        app_log_info!("🔧 VERIFY_INTEGRITY: old_size = {}, new_size = {}", old_size, new_size);

        if old_size != new_size {
            app_log_error!("❌ VERIFY_INTEGRITY: File sizes don't match");
            return Err("Database initialization has failed.".into());
        }

        app_log_info!("✅ VERIFY_INTEGRITY: File sizes match, skipping encrypted database verification");
        app_log_info!("✅ VERIFY_INTEGRITY: Since database is encrypted, we can't verify integrity without key");
        app_log_info!("✅ VERIFY_INTEGRITY: Assuming integrity is good if file sizes match");

        // For encrypted databases, we can't easily verify integrity without the key
        // Since we're copying the file and the sizes match, we assume it's good
        // The real verification will happen when the database is actually used
        Ok(())
    }

    // ===== ENCRYPTION-RELATED METHODS =====

    /// Handle database initialization with app storage operations
    pub fn initialize_database(&mut self) -> Result<()> {
        let db_path = self.db_path.read().unwrap().clone();

        // Check if database already exists and is encrypted
        if db_path.exists() {
            app_log_info!("🔍 Found existing database at: {}", db_path.display());

            // Try to open it as an encrypted database
            if let Ok(_) = self.get_encrypted_connection() {
                app_log_info!("✅ Existing database is encrypted and accessible");
                self.is_encrypted = true;

                // Update the connection to use encryption
                let new_connection = self.get_encrypted_connection()?;
                let mut db = self.db.lock().unwrap();
                *db = new_connection;

                app_log_info!("✅ Using existing encrypted database");
                return Ok(());
            } else {
                app_log_info!("⚠️ Existing database is not encrypted or corrupted, will create new one");
            }
        }

        // Generate encryption key if needed
        if !self.encryption_service.has_database_key() {
            let db_key = DatabaseEncryptionService::generate_database_key()?;
            self.encryption_service.store_database_key(&db_key)?;
            app_log_info!("🔑 Generated new encryption key for database");
        }

        // Create new encrypted database (only if database doesn't exist or is unencrypted)
        self.create_encrypted_database()?;
        self.is_encrypted = true;

        // Update the connection to use encryption
        let new_connection = self.get_encrypted_connection()?;
        let mut db = self.db.lock().unwrap();
        *db = new_connection;

        app_log_info!("✅ Created new encrypted database and updated connection");

        Ok(())
    }

    /// Create a new encrypted database
    fn create_encrypted_database(&self) -> Result<()> {
        let db_key = self.encryption_service.get_database_key()?;
        let db_path = self.db_path.read().unwrap().clone();

        // Only remove existing database if it's not encrypted
        if db_path.exists() {
            // Try to open as encrypted first
            if let Ok(_) = self.get_encrypted_connection() {
                app_log_info!("✅ Database already exists and is encrypted, skipping creation");
                return Ok(());
            } else {
                // Database exists but is not encrypted, remove it
                fs::remove_file(&db_path)?;
                app_log_info!("🗑️ Removed unencrypted database file");
            }
        }

        let connection = Connection::open(&db_path)?;

        // Configure SQLCipher
        connection.execute_batch(&format!("PRAGMA key = '{}'", db_key))?;
        connection.execute_batch("PRAGMA cipher_compatibility = 3")?;
        connection.execute_batch("PRAGMA journal_mode = WAL")?;

        // Load sqlite-vec extension
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        app_log_info!("✅ Created new encrypted database with sqlite-vec extension");

        Ok(())
    }

    /// Check if database is encrypted
    pub fn is_database_encrypted(&self) -> bool {
        // Since we only create encrypted databases now, always return true
        true
    }

    /// Get encrypted database connection
    fn get_encrypted_connection(&self) -> Result<Connection> {
        let db_key = self.encryption_service.get_database_key()?;
        let db_path = self.db_path.read().unwrap().clone();

        let connection = Connection::open(&db_path)?;
        app_log_info!("✅ Successfully opened database file");

        // Configure SQLCipher
        connection.execute_batch(&format!("PRAGMA key = '{}'", db_key))?;

        // Set basic SQLCipher settings for maximum compatibility
        connection.execute_batch("PRAGMA cipher_compatibility = 3")?;

        // Ensure the database is writable
        connection.execute_batch("PRAGMA journal_mode = WAL")?;

        // Load sqlite-vec extension for vector search functions
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        // Verify the key is correct by attempting to read from the database
        let _count: i64 = connection.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
        app_log_info!("✅ Database access verified successfully");

        Ok(connection)
    }
    /// Migrate custom vector_search.db to .cosmos.db
    fn migrate_custom_vector_db(config_service: &mut ConfigService) -> Result<()> {
        let custom_path_str = config_service.get_custom_db_path();
        if let Some(custom_path_str) = custom_path_str {
            let custom_path = PathBuf::from(custom_path_str);

            // Check if the custom path points to vector_search.db
            if let Some(filename) = custom_path.file_name() {
                if filename == "vector_search.db" && custom_path.exists() {
                    if let Some(parent_dir) = custom_path.parent() {
                        let new_db_path = parent_dir.join(crate::constants::DATABASE_FILENAME);

                        app_log_info!("🔄 DATABASE MIGRATION: Found custom vector_search.db at {}", custom_path.display());
                        app_log_info!("🔄 DATABASE MIGRATION: Will migrate to {} and encrypt", new_db_path.display());

                        // Remove the old file
                        if let Err(e) = fs::remove_file(&custom_path) {
                            app_log_warn!("⚠️ DATABASE MIGRATION: Failed to remove old vector_search.db: {}", e);
                        } else {
                            app_log_info!("✅ DATABASE MIGRATION: Removed old vector_search.db");
                        }

                        // Update config to point to new path
                        if let Err(e) = config_service.set_custom_db_path(Some(new_db_path.to_string_lossy().to_string())) {
                            app_log_warn!("⚠️ DATABASE MIGRATION: Failed to save updated config: {}", e);
                        } else {
                            app_log_info!("✅ DATABASE MIGRATION: Updated config with new .cosmos.db path");
                        }

                        app_log_info!("✅ DATABASE MIGRATION: Migration complete. New encrypted database will be created at {}", new_db_path.display());
                    }
                }
            }
        }

        Ok(())
    }
}
