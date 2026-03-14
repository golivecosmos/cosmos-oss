use crate::services::database_service::DatabaseService;
use crate::{app_log_debug, app_log_error, app_log_info, app_log_warn};
use anyhow::{anyhow, Result};
use rusqlite::Connection;
use std::sync::Arc;

/// Schema Management Service
///
/// Handles all database schema operations including:
/// - Table creation and schema validation
/// - Index management
/// - Schema metadata handling
/// - Migration coordination
pub struct SchemaService {
    db_service: Arc<DatabaseService>,
}

const SCHEMA_VERSION: &str = "2";

impl SchemaService {
    /// Create a new SchemaService instance
    pub fn new(db_service: Arc<DatabaseService>) -> Self {
        Self { db_service }
    }

    /// Handle schema setup - coordinates the entire schema initialization process
    pub fn handle_schema_setup(&self) -> Result<()> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        app_log_info!("🔧 SCHEMA SETUP: Starting database schema validation and setup");

        // Check if we have a Nomic-compatible database
        let has_nomic_schema = match self.check_nomic_schema(&db) {
            Ok(result) => result,
            Err(e) => {
                app_log_warn!(
                    "⚠️ SCHEMA CHECK: Failed to check Nomic schema: {}, assuming false",
                    e
                );
                false
            }
        };

        if has_nomic_schema {
            app_log_info!("✅ SCHEMA: Database already has Nomic-compatible schema");
            // **NEW: Even with Nomic schema, ensure jobs table exists (backwards compatibility)**
            match self.ensure_jobs_table_exists(&db) {
                Ok(_) => app_log_info!("✅ SCHEMA: Jobs table verified/created successfully"),
                Err(e) => {
                    app_log_error!("❌ SCHEMA: Failed to ensure jobs table exists: {}", e);
                    return Err(e);
                }
            }

            // **NEW: Ensure generations table exists (backwards compatibility)**
            match self.ensure_generations_table_exists(&db) {
                Ok(_) => {
                    app_log_info!("✅ SCHEMA: Generations table verified/created successfully")
                }
                Err(e) => {
                    app_log_error!(
                        "❌ SCHEMA: Failed to ensure generations table exists: {}",
                        e
                    );
                    return Err(e);
                }
            }

            // **NEW: Ensure app tables exist (backwards compatibility)**
            match self.ensure_app_tables_exist(&db) {
                Ok(_) => app_log_info!("✅ SCHEMA: App tables verified/created successfully"),
                Err(e) => {
                    app_log_error!("❌ SCHEMA: Failed to ensure app tables exist: {}", e);
                    return Err(e);
                }
            }

            // **NEW: Ensure watched folder tables exist (backwards compatibility)**
            match self.ensure_watched_folders_tables_exist(&db) {
                Ok(_) => app_log_info!("✅ SCHEMA: Watched folder tables verified/created successfully"),
                Err(e) => {
                    app_log_error!(
                        "❌ SCHEMA: Failed to ensure watched folder tables exist: {}",
                        e
                    );
                    return Err(e);
                }
            }
        } else {
            // Check if any tables exist (old versionless database)
            let has_existing_tables = match self.check_existing_tables(&db) {
                Ok(result) => result,
                Err(e) => {
                    app_log_warn!(
                        "⚠️ SCHEMA CHECK: Failed to check existing tables: {}, assuming false",
                        e
                    );
                    false
                }
            };

            if has_existing_tables {
                app_log_warn!("🔄 SCHEMA: Detected old versionless database - recreating for Nomic compatibility");
                app_log_warn!("📝 SCHEMA: Old embeddings incompatible with Nomic models - clean slate required");
                match self.recreate_database_for_upgrade(&db) {
                    Ok(_) => app_log_info!(
                        "✅ SCHEMA: Database successfully recreated for Nomic compatibility"
                    ),
                    Err(e) => {
                        app_log_error!("❌ SCHEMA: Failed to recreate database: {}", e);
                        return Err(e);
                    }
                }
            } else {
                app_log_info!("🆕 SCHEMA: Creating new Nomic-compatible database");
                match self.create_fresh_schema(&db) {
                    Ok(_) => app_log_info!(
                        "✅ SCHEMA: Fresh Nomic-compatible database created successfully"
                    ),
                    Err(e) => {
                        app_log_error!("❌ SCHEMA: Failed to create fresh schema: {}", e);
                        return Err(e);
                    }
                }
            }
        }

        app_log_info!("✅ SCHEMA SETUP: Database schema setup completed successfully");

        Ok(())
    }

    /// Check if database has Nomic-compatible schema
    pub fn check_nomic_schema(&self, db: &Connection) -> Result<bool> {
        // Check if schema_info table exists with Nomic model metadata
        let has_schema_info = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_info'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !has_schema_info {
            return Ok(false);
        }

        // Check if it has Nomic model metadata
        let has_nomic_metadata = match db.query_row(
            "SELECT COUNT(*) FROM schema_info WHERE key = 'embedding_model' AND value = 'nomic-embed-v1.5'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let has_schema_version = match db.query_row(
            "SELECT COUNT(*) FROM schema_info WHERE key = 'schema_version' AND value = ?1",
            rusqlite::params![SCHEMA_VERSION],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let has_text_chunks_table = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='text_chunks'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let has_vec_text_chunks_table = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='vec_text_chunks'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        Ok(has_nomic_metadata
            && has_schema_version
            && has_text_chunks_table
            && has_vec_text_chunks_table)
    }

    /// Check if database has any existing tables
    pub fn check_existing_tables(&self, db: &Connection) -> Result<bool> {
        let table_count = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('images', 'vec_images')",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count,
            Err(_) => 0,
        };

        Ok(table_count > 0)
    }

    /// Create a fresh database schema for Nomic models
    pub fn create_fresh_schema(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ SCHEMA: Creating fresh Nomic-compatible database schema");

        // Create main images table optimized for Nomic embeddings
        self.create_nomic_images_table(db)?;
        self.create_text_chunks_table(db)?;

        // Create jobs table for persistent job tracking
        self.create_jobs_table(db)?;

        // Create drive tracking tables
        self.create_drives_tables(db)?;

        // Create transcriptions table for audio content
        self.create_transcriptions_table(db)?;

        // Create app installation tables
        self.create_app_tables(db)?;

        // Create generations table for video generation tracking
        self.create_generations_table(db)?;

        // Create watched folder tables for background indexing
        self.create_watched_folders_tables(db)?;

        // Create indexes
        self.create_indexes(db)?;

        // Create virtual table for Nomic 768-dim embeddings
        self.create_nomic_virtual_table(db)?;
        self.create_text_chunks_virtual_table(db)?;

        // Set schema metadata
        self.set_schema_metadata(db)?;

        app_log_info!("✅ SCHEMA: Fresh Nomic-compatible database created successfully");
        Ok(())
    }

    /// Create watched folders tables for background indexing orchestration
    pub fn create_watched_folders_tables(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ WATCHED FOLDERS: Creating watched folder tables");

        db.execute(
            "CREATE TABLE IF NOT EXISTS watched_folders (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                recursive INTEGER NOT NULL DEFAULT 1,
                enabled INTEGER NOT NULL DEFAULT 1,
                auto_transcribe_videos INTEGER NOT NULL DEFAULT 1,
                status TEXT NOT NULL DEFAULT 'idle',
                last_scan_at TEXT,
                last_event_at TEXT,
                metadata TEXT DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            rusqlite::params![],
        )?;

        db.execute(
            "CREATE TABLE IF NOT EXISTS watched_folder_file_state (
                watched_folder_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER,
                mtime_unix_ms INTEGER,
                content_sig TEXT,
                last_seen_at TEXT NOT NULL,
                last_index_job_id TEXT,
                PRIMARY KEY (watched_folder_id, file_path),
                FOREIGN KEY (watched_folder_id) REFERENCES watched_folders(id) ON DELETE CASCADE
            )",
            rusqlite::params![],
        )?;

        let indexes = [
            (
                "idx_watched_folders_enabled",
                "CREATE INDEX IF NOT EXISTS idx_watched_folders_enabled ON watched_folders(enabled)",
            ),
            (
                "idx_watched_folders_status",
                "CREATE INDEX IF NOT EXISTS idx_watched_folders_status ON watched_folders(status)",
            ),
            (
                "idx_watched_folder_file_state_path",
                "CREATE INDEX IF NOT EXISTS idx_watched_folder_file_state_path ON watched_folder_file_state(file_path)",
            ),
        ];

        for (index_name, sql) in indexes {
            match db.execute(sql, rusqlite::params![]) {
                Ok(_) => app_log_debug!(
                    "✅ INDEX: Created watched folder table index: {}",
                    index_name
                ),
                Err(e) => {
                    app_log_warn!(
                        "⚠️ INDEX: Failed to create watched folder table index {}: {}",
                        index_name,
                        e
                    );
                }
            }
        }

        app_log_info!("✅ WATCHED FOLDERS: Watched folder tables ready");
        Ok(())
    }

    /// Ensure watched folder tables exist for backwards compatibility.
    pub fn ensure_watched_folders_tables_exist(&self, db: &Connection) -> Result<()> {
        let table_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='watched_folders'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !table_exists {
            app_log_warn!(
                "⚠️ WATCHED FOLDERS: Missing watched folder tables, creating for backwards compatibility"
            );
            self.create_watched_folders_tables(db)?;
            app_log_info!("✅ WATCHED FOLDERS: Watched folder tables created successfully");
        } else {
            // Keep indexes current.
            self.create_watched_folders_tables(db)?;
        }

        Ok(())
    }

    /// Recreate database for version upgrade (clean slate)
    pub fn recreate_database_for_upgrade(&self, db: &Connection) -> Result<()> {
        app_log_warn!("🗑️ SCHEMA: Recreating database for Nomic model upgrade");
        app_log_warn!(
            "💡 REASON: Embedding dimensions/models changed - old embeddings incompatible"
        );

        // Drop all existing tables (clean slate)
        let _ = db.execute("DROP TABLE IF EXISTS vec_images", rusqlite::params![]);
        let _ = db.execute("DROP TABLE IF EXISTS vec_text_chunks", rusqlite::params![]);
        let _ = db.execute("DROP TABLE IF EXISTS images", rusqlite::params![]);
        let _ = db.execute("DROP TABLE IF EXISTS text_chunks", rusqlite::params![]);
        let _ = db.execute("DROP TABLE IF EXISTS schema_info", rusqlite::params![]);

        // Create fresh schema for Nomic models
        self.create_fresh_schema(db)?;

        app_log_warn!("⚠️ SCHEMA: Database recreated for Nomic compatibility");
        app_log_warn!("📝 SCHEMA: Users will need to re-index files with new Nomic embeddings");

        Ok(())
    }

    /// Create images table optimized for Nomic embeddings
    pub fn create_nomic_images_table(&self, db: &Connection) -> Result<()> {
        db.execute(
            "CREATE TABLE images (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                parent_file_path TEXT,
                file_name TEXT,
                mime_type TEXT,
                width INTEGER,
                height INTEGER,
                aspect_ratio REAL,
                fs_size INTEGER,
                created_at TEXT,
                updated_at TEXT,
                last_indexed_at TEXT,
                status TEXT DEFAULT 'indexed',
                tags TEXT DEFAULT '',

                -- Video frame specific fields
                source_type TEXT,
                timestamp REAL,
                timestamp_formatted TEXT,
                frame_number INTEGER,
                video_duration REAL,

                -- Drive support fields
                drive_uuid TEXT,
                relative_path TEXT,

                -- Metadata as JSON
                metadata TEXT,

                -- Nomic embedding (768 dimensions * 4 bytes = 3072 bytes)
                -- Guaranteed to be from nomic-embed-v1.5 models
                embedding BLOB,

                -- Foreign key relationship to drives table
                FOREIGN KEY (drive_uuid) REFERENCES drives(uuid) ON DELETE SET NULL
            )",
            rusqlite::params![],
        )?;

        app_log_info!("✅ SCHEMA: Nomic-optimized images table created (768-dim embeddings)");
        Ok(())
    }

    /// Create text chunk table for semantic document retrieval
    pub fn create_text_chunks_table(&self, db: &Connection) -> Result<()> {
        db.execute(
            "CREATE TABLE text_chunks (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                parent_file_path TEXT,
                file_name TEXT,
                mime_type TEXT,
                chunk_index INTEGER NOT NULL,
                chunk_text TEXT NOT NULL,
                char_start INTEGER,
                char_end INTEGER,
                token_estimate INTEGER,
                metadata TEXT,
                embedding BLOB,
                drive_uuid TEXT,
                created_at TEXT,
                updated_at TEXT,
                last_indexed_at TEXT,
                FOREIGN KEY (drive_uuid) REFERENCES drives(uuid) ON DELETE SET NULL
            )",
            rusqlite::params![],
        )?;

        app_log_info!("✅ SCHEMA: text_chunks table created (chunk-level text embeddings)");
        Ok(())
    }

    /// Create virtual table optimized for Nomic embeddings
    pub fn create_nomic_virtual_table(&self, db: &Connection) -> Result<()> {
        let create_vec_table_result = db.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_images USING vec0(
                embedding float[768] distance_metric=cosine
            )",
            rusqlite::params![],
        );

        match create_vec_table_result {
            Ok(_) => {
                app_log_info!(
                    "✅ SQLITE: Nomic-optimized vector search table created (768-dim, cosine)"
                );
            }
            Err(e) => {
                app_log_warn!("⚠️ SQLITE: Could not create vector table: {}", e);
                app_log_info!(
                    "📝 SQLITE: Will use manual distance calculations for Nomic embeddings"
                );
            }
        }

        Ok(())
    }

    /// Create vector table for text chunk embeddings
    pub fn create_text_chunks_virtual_table(&self, db: &Connection) -> Result<()> {
        let create_vec_table_result = db.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_text_chunks USING vec0(
                embedding float[768] distance_metric=cosine
            )",
            rusqlite::params![],
        );

        match create_vec_table_result {
            Ok(_) => app_log_info!("✅ SQLITE: text chunk vector table created (768-dim, cosine)"),
            Err(e) => {
                app_log_error!("❌ SQLITE: Failed to create text chunk vector table: {}", e);
                return Err(anyhow!("Failed to create text chunk vector table: {}", e));
            }
        }

        Ok(())
    }

    /// Set schema metadata for Nomic model
    pub fn set_schema_metadata(&self, db: &Connection) -> Result<()> {
        // Create schema_info table
        db.execute(
            "CREATE TABLE IF NOT EXISTS schema_info (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            rusqlite::params![],
        )?;

        let now = chrono::Utc::now().to_rfc3339();

        // Store app version
        let app_version = env!("CARGO_PKG_VERSION");
        db.execute(
            "INSERT OR REPLACE INTO schema_info (key, value, created_at, updated_at)
             VALUES ('app_version', ?, ?, ?)",
            rusqlite::params![app_version, now, now],
        )?;

        db.execute(
            "INSERT OR REPLACE INTO schema_info (key, value, created_at, updated_at)
             VALUES ('schema_version', ?, ?, ?)",
            rusqlite::params![SCHEMA_VERSION, now, now],
        )?;

        // **Record Nomic model info**
        db.execute(
            "INSERT OR REPLACE INTO schema_info (key, value, created_at, updated_at)
             VALUES ('embedding_model', ?, ?, ?)",
            rusqlite::params!["nomic-embed-v1.5", now, now],
        )?;

        db.execute(
            "INSERT OR REPLACE INTO schema_info (key, value, created_at, updated_at)
             VALUES ('embedding_dimensions', ?, ?, ?)",
            rusqlite::params!["768", now, now],
        )?;

        app_log_info!("📊 SCHEMA: Nomic model metadata recorded");
        Ok(())
    }

    /// Create database indexes
    pub fn create_indexes(&self, db: &Connection) -> Result<()> {
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_path ON images(file_path)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_source_type ON images(source_type)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON images(created_at)",
            rusqlite::params![],
        )?;

        // Drive-related indexes
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_images_drive_uuid ON images(drive_uuid)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_images_relative_path ON images(relative_path)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_text_chunks_file_path ON text_chunks(file_path)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_text_chunks_mime_type ON text_chunks(mime_type)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_text_chunks_drive_uuid ON text_chunks(drive_uuid)",
            rusqlite::params![],
        )?;

        Ok(())
    }

    /// Ensure jobs table exists (backwards compatibility)
    pub fn ensure_jobs_table_exists(&self, db: &Connection) -> Result<()> {
        // Check if jobs table exists
        let table_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='jobs'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !table_exists {
            app_log_warn!(
                "⚠️ JOBS TABLE: Missing jobs table, creating for backwards compatibility"
            );
            self.create_jobs_table(db)?;
            app_log_info!("✅ JOBS TABLE: Jobs table created successfully");
        } else {
            app_log_debug!("✅ JOBS TABLE: Jobs table already exists");
            // Keep indexes current even on existing databases.
            self.create_jobs_indexes(db)?;
        }

        // Also ensure app tables exist for backwards compatibility
        self.ensure_app_tables_exist(db)?;

        Ok(())
    }

    /// Check if jobs table exists (defensive method)
    pub fn jobs_table_exists(&self, db: &Connection) -> bool {
        match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='jobs'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    /// Create jobs table for persistent job tracking
    pub fn create_jobs_table(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ JOBS TABLE: Creating jobs table for persistent job tracking");

        // Check if drives table exists before adding foreign key constraint
        let drives_table_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='drives'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let foreign_key_constraint = if drives_table_exists {
            ",\n                -- Foreign key relationship to drives table\n                FOREIGN KEY (drive_uuid) REFERENCES drives(uuid) ON DELETE SET NULL"
        } else {
            ""
        };

        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                job_type TEXT NOT NULL,  -- 'file', 'directory', 'video'
                target_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'completed', 'failed', 'cancelled'
                current_file TEXT,
                processed INTEGER DEFAULT 0,
                total INTEGER DEFAULT 0,
                errors TEXT DEFAULT '[]', -- JSON array of error messages
                failed_files TEXT DEFAULT '[]', -- JSON array of failed file info
                metadata TEXT DEFAULT '{{}}', -- JSON for job-specific data
                retry_count INTEGER DEFAULT 0, -- Number of retry attempts
                max_retries INTEGER DEFAULT 3, -- Maximum automatic retries
                next_retry_at TEXT, -- When to retry next (for exponential backoff)

                -- Drive support fields
                drive_uuid TEXT,
                relative_path TEXT,

                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                updated_at TEXT NOT NULL{}
            )",
            foreign_key_constraint
        );

        let create_result = db.execute(&create_sql, rusqlite::params![]);

        match create_result {
            Ok(_) => app_log_info!("✅ JOBS TABLE: Jobs table created successfully"),
            Err(e) => {
                app_log_error!("❌ JOBS TABLE: Failed to create jobs table: {}", e);
                return Err(anyhow!("Failed to create jobs table: {}", e));
            }
        }

        self.create_jobs_indexes(db)?;

        app_log_info!(
            "✅ SCHEMA: Jobs table created for persistent job tracking with retry support"
        );
        Ok(())
    }

    fn create_jobs_indexes(&self, db: &Connection) -> Result<()> {
        // Replace legacy uniqueness index (target_path only) with type-aware uniqueness.
        let _ = db.execute(
            "DROP INDEX IF EXISTS idx_jobs_unique_active_target",
            rusqlite::params![],
        );

        let indexes = [
            ("idx_jobs_status", "CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)"),
            ("idx_jobs_created_at", "CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at)"),
            ("idx_jobs_target_path", "CREATE INDEX IF NOT EXISTS idx_jobs_target_path ON jobs(target_path)"),
            ("idx_jobs_next_retry_at", "CREATE INDEX IF NOT EXISTS idx_jobs_next_retry_at ON jobs(next_retry_at)"),
            ("idx_jobs_drive_uuid", "CREATE INDEX IF NOT EXISTS idx_jobs_drive_uuid ON jobs(drive_uuid)"),
            (
                "idx_jobs_unique_active_target_type",
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_unique_active_target_type ON jobs(job_type, target_path) WHERE status IN ('pending', 'running')"
            ),
        ];

        for (index_name, sql) in indexes.iter() {
            match db.execute(sql, rusqlite::params![]) {
                Ok(_) => app_log_debug!("✅ INDEX: Created jobs table index: {}", index_name),
                Err(e) => {
                    app_log_warn!(
                        "⚠️ INDEX: Failed to create jobs table index {}: {}",
                        index_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Create drive tracking tables
    pub fn create_drives_tables(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ DRIVES: Creating drive tracking tables");

        // Create drives table
        let create_drives_result = db.execute(
            "CREATE TABLE IF NOT EXISTS drives (
                uuid TEXT PRIMARY KEY,
                name TEXT NOT NULL,  -- System detected name
                custom_name TEXT,    -- User-assigned custom name
                physical_location TEXT,  -- User-assigned physical storage location
                last_mount_path TEXT,
                total_space INTEGER DEFAULT 0,
                free_space INTEGER DEFAULT 0,
                is_removable BOOLEAN DEFAULT 1,
                first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
                status TEXT DEFAULT 'connected', -- connected, disconnected, indexing, error
                indexed_files_count INTEGER DEFAULT 0,
                total_size_indexed INTEGER DEFAULT 0,
                metadata TEXT DEFAULT '{}' -- JSON for additional drive metadata
            )",
            rusqlite::params![],
        );

        match create_drives_result {
            Ok(_) => app_log_info!("✅ DRIVES: Drives table created successfully"),
            Err(e) => {
                app_log_error!("❌ DRIVES: Failed to create drives table: {}", e);
                return Err(anyhow!("Failed to create drives table: {}", e));
            }
        }

        // Create drive mount history table
        let create_mounts_result = db.execute(
            "CREATE TABLE IF NOT EXISTS drive_mounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                drive_uuid TEXT NOT NULL,
                mount_path TEXT NOT NULL,
                mounted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                unmounted_at DATETIME,
                FOREIGN KEY (drive_uuid) REFERENCES drives(uuid) ON DELETE CASCADE
            )",
            rusqlite::params![],
        );

        match create_mounts_result {
            Ok(_) => app_log_info!("✅ DRIVES: Drive mounts table created successfully"),
            Err(e) => {
                app_log_error!("❌ DRIVES: Failed to create drive_mounts table: {}", e);
                return Err(anyhow!("Failed to create drive_mounts table: {}", e));
            }
        }

        // Create indexes for drive tables
        let drive_indexes = [
            (
                "idx_drives_status",
                "CREATE INDEX IF NOT EXISTS idx_drives_status ON drives(status)",
            ),
            (
                "idx_drives_last_seen",
                "CREATE INDEX IF NOT EXISTS idx_drives_last_seen ON drives(last_seen)",
            ),
            (
                "idx_drive_mounts_uuid",
                "CREATE INDEX IF NOT EXISTS idx_drive_mounts_uuid ON drive_mounts(drive_uuid)",
            ),
            (
                "idx_drive_mounts_path",
                "CREATE INDEX IF NOT EXISTS idx_drive_mounts_path ON drive_mounts(mount_path)",
            ),
        ];

        for (index_name, sql) in drive_indexes.iter() {
            match db.execute(sql, rusqlite::params![]) {
                Ok(_) => app_log_debug!("✅ INDEX: Created drive table index: {}", index_name),
                Err(e) => {
                    app_log_warn!(
                        "⚠️ INDEX: Failed to create drive table index {}: {}",
                        index_name,
                        e
                    );
                    // Don't fail the entire operation for index creation failures
                }
            }
        }

        app_log_info!("✅ DRIVES: Drive tracking tables created successfully");

        // Note: Drive schema migrations are now handled by the migration service

        Ok(())
    }

    /// Create transcriptions table for audio content
    pub fn create_transcriptions_table(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ TRANSCRIPTIONS: Creating transcriptions table for audio content");

        // Check if drives table exists before adding foreign key constraint
        let drives_table_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='drives'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        let foreign_key_constraint = if drives_table_exists {
            ",\n                FOREIGN KEY (drive_uuid) REFERENCES drives(uuid) ON DELETE SET NULL"
        } else {
            ""
        };

        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS transcriptions (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                parent_file_path TEXT,
                file_name TEXT,
                mime_type TEXT,
                duration_seconds REAL,
                fs_size INTEGER,
                created_at TEXT,
                last_modified TEXT,
                last_indexed_at TEXT,
                status TEXT DEFAULT 'completed',
                transcription_text TEXT NOT NULL,
                segments TEXT, -- JSON array of transcription segments with timestamps
                language TEXT,
                model_name TEXT,
                confidence_score REAL,

                -- Drive tracking (same as images table)
                drive_uuid TEXT{}
            )",
            foreign_key_constraint
        );

        let create_transcriptions_result = db.execute(&create_sql, rusqlite::params![]);

        match create_transcriptions_result {
            Ok(_) => app_log_info!("✅ TRANSCRIPTIONS: Transcriptions table created successfully"),
            Err(e) => {
                app_log_error!(
                    "❌ TRANSCRIPTIONS: Failed to create transcriptions table: {}",
                    e
                );
                return Err(anyhow::anyhow!(
                    "Failed to create transcriptions table: {}",
                    e
                ));
            }
        }

        app_log_info!("✅ TRANSCRIPTIONS: Audio transcription storage ready");
        Ok(())
    }

    /// Create app installation tables
    pub fn create_app_tables(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ APP INSTALLATION: Creating app installation tables");

        // Create app_installations table
        let create_app_installations_result = db.execute(
            "CREATE TABLE IF NOT EXISTS app_installations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_name TEXT NOT NULL,
                app_version TEXT NOT NULL,
                installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                metadata TEXT DEFAULT '{}' -- JSON for additional app metadata
            )",
            rusqlite::params![],
        );

        match create_app_installations_result {
            Ok(_) => {
                app_log_info!("✅ APP INSTALLATION: App installations table created successfully")
            }
            Err(e) => {
                app_log_error!(
                    "❌ APP INSTALLATION: Failed to create app_installations table: {}",
                    e
                );
                return Err(anyhow!("Failed to create app_installations table: {}", e));
            }
        }

        // Create app_settings table
        let create_app_settings_result = db.execute(
            "CREATE TABLE IF NOT EXISTS app_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_id INTEGER NOT NULL,
                setting_key TEXT NOT NULL,
                setting_value TEXT NOT NULL,
                setting_type TEXT NOT NULL, -- 'string', 'number', 'boolean', 'json'
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (app_id) REFERENCES app_installations(id) ON DELETE CASCADE
            )",
            rusqlite::params![],
        );

        match create_app_settings_result {
            Ok(_) => app_log_info!("✅ APP INSTALLATION: App settings table created successfully"),
            Err(e) => {
                app_log_error!(
                    "❌ APP INSTALLATION: Failed to create app_settings table: {}",
                    e
                );
                return Err(anyhow!("Failed to create app_settings table: {}", e));
            }
        }

        // Create app_logs table
        let create_app_logs_result = db.execute(
            "CREATE TABLE IF NOT EXISTS app_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                log_level TEXT NOT NULL, -- 'debug', 'info', 'warn', 'error'
                log_message TEXT NOT NULL,
                log_source TEXT NOT NULL,
                log_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            rusqlite::params![],
        );

        match create_app_logs_result {
            Ok(_) => app_log_info!("✅ APP INSTALLATION: App logs table created successfully"),
            Err(e) => {
                app_log_error!(
                    "❌ APP INSTALLATION: Failed to create app_logs table: {}",
                    e
                );
                return Err(anyhow!("Failed to create app_logs table: {}", e));
            }
        }

        // Create indexes for app tables
        let app_indexes = [
            ("idx_app_installations_app_name", "CREATE INDEX IF NOT EXISTS idx_app_installations_app_name ON app_installations(app_name)"),
            ("idx_app_installations_app_version", "CREATE INDEX IF NOT EXISTS idx_app_installations_app_version ON app_installations(app_version)"),
            ("idx_app_settings_app_id", "CREATE INDEX IF NOT EXISTS idx_app_settings_app_id ON app_settings(app_id)"),
            ("idx_app_settings_key", "CREATE INDEX IF NOT EXISTS idx_app_settings_key ON app_settings(setting_key)"),
            ("idx_app_logs_level", "CREATE INDEX IF NOT EXISTS idx_app_logs_level ON app_logs(log_level)"),
            ("idx_app_logs_source", "CREATE INDEX IF NOT EXISTS idx_app_logs_source ON app_logs(log_source)"),
        ];

        for (index_name, sql) in app_indexes.iter() {
            match db.execute(sql, rusqlite::params![]) {
                Ok(_) => app_log_debug!("✅ INDEX: Created app table index: {}", index_name),
                Err(e) => {
                    app_log_warn!(
                        "⚠️ INDEX: Failed to create app table index {}: {}",
                        index_name,
                        e
                    );
                    // Don't fail the entire operation for index creation failures
                }
            }
        }

        app_log_info!("✅ APP INSTALLATION: App installation tables created successfully");
        Ok(())
    }

    /// Create generations table for video generation tracking
    pub fn create_generations_table(&self, db: &Connection) -> Result<()> {
        app_log_info!("🏗️ GENERATIONS: Creating generations table for video generation tracking");

        let create_generations_result = db.execute(
            "CREATE TABLE IF NOT EXISTS generations (
                id TEXT PRIMARY KEY,
                user_prompt TEXT NOT NULL,
                json_prompt TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'veo3',
                generated_file_path TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            rusqlite::params![],
        );

        match create_generations_result {
            Ok(_) => app_log_info!("✅ GENERATIONS: Generations table created successfully"),
            Err(e) => {
                app_log_error!("❌ GENERATIONS: Failed to create generations table: {}", e);
                return Err(anyhow!("Failed to create generations table: {}", e));
            }
        }

        // Create indexes for generations table
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_generations_created_at ON generations(created_at)",
            rusqlite::params![],
        )?;
        db.execute(
            "CREATE INDEX IF NOT EXISTS idx_generations_source ON generations(source)",
            rusqlite::params![],
        )?;
        db.execute("CREATE INDEX IF NOT EXISTS idx_generations_file_path ON generations(generated_file_path)", rusqlite::params![])?;

        app_log_info!("✅ GENERATIONS: Video generation tracking ready");
        Ok(())
    }

    /// Ensure generations table exists (backwards compatibility)
    pub fn ensure_generations_table_exists(&self, db: &Connection) -> Result<()> {
        // Check if generations table exists
        let table_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='generations'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !table_exists {
            app_log_warn!(
                "⚠️ GENERATIONS: Missing generations table, creating for backwards compatibility"
            );
            self.create_generations_table(db)?;
            app_log_info!("✅ GENERATIONS: Generations table created successfully");
        } else {
            app_log_debug!("✅ GENERATIONS: Generations table already exists");
        }

        Ok(())
    }

    /// Ensure app tables exist for backwards compatibility
    pub fn ensure_app_tables_exist(&self, db: &Connection) -> Result<()> {
        // Check if app_installations table exists
        let app_installations_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_installations'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !app_installations_exists {
            app_log_warn!("⚠️ APP INSTALLATION: Missing app_installations table, creating for backwards compatibility");
            self.create_app_tables(db)?;
            app_log_info!("✅ APP INSTALLATION: App installations table created successfully");
        } else {
            app_log_debug!("✅ APP INSTALLATION: App installations table already exists");
        }

        // Check if app_settings table exists
        let app_settings_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_settings'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !app_settings_exists {
            app_log_warn!("⚠️ APP INSTALLATION: Missing app_settings table, creating for backwards compatibility");
            self.create_app_tables(db)?;
            app_log_info!("✅ APP INSTALLATION: App settings table created successfully");
        } else {
            // Check if app_id column exists in app_settings table
            let app_id_column_exists = match db.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('app_settings') WHERE name='app_id'",
                rusqlite::params![],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(count) => count > 0,
                Err(_) => false,
            };

            if !app_id_column_exists {
                app_log_warn!("⚠️ APP INSTALLATION: Missing app_id column in app_settings table, adding for backwards compatibility");
                // Add app_id column to existing table
                db.execute(
                    "ALTER TABLE app_settings ADD COLUMN app_id INTEGER",
                    rusqlite::params![],
                )?;
                app_log_info!("✅ APP INSTALLATION: Added app_id column to app_settings table");
            } else {
                app_log_debug!("✅ APP INSTALLATION: App settings table already has app_id column");
            }
        }

        // Check if app_logs table exists
        let app_logs_exists = match db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='app_logs'",
            rusqlite::params![],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(count) => count > 0,
            Err(_) => false,
        };

        if !app_logs_exists {
            app_log_warn!(
                "⚠️ APP INSTALLATION: Missing app_logs table, creating for backwards compatibility"
            );
            self.create_app_tables(db)?;
            app_log_info!("✅ APP INSTALLATION: App logs table created successfully");
        } else {
            app_log_debug!("✅ APP INSTALLATION: App logs table already exists");
        }

        Ok(())
    }

    /// Get comprehensive schema information including Nomic compatibility check
    pub fn get_schema_info(&self) -> Result<serde_json::Value> {
        let connection = self.db_service.get_connection();
        let db = connection.lock().unwrap();

        let mut info = std::collections::HashMap::new();

        // Get all schema info (if it exists)
        if let Ok(mut stmt) = db.prepare("SELECT key, value, updated_at FROM schema_info") {
            let rows = stmt.query_map(rusqlite::params![], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;

            for row in rows {
                let (key, value, updated_at) = row?;
                info.insert(
                    key,
                    serde_json::json!({
                        "value": value,
                        "updated_at": updated_at
                    }),
                );
            }
        }

        // Check if this is a Nomic-compatible database
        let has_nomic_schema = self.check_nomic_schema(&db).unwrap_or(false);

        Ok(serde_json::json!({
            "has_nomic_schema": has_nomic_schema,
            "schema_info": info,
            "strategy": "simple_nomic_or_recreate",
            "embedding_model": "nomic-embed-v1.5",
            "embedding_dimensions": 768,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_schema_service_creation() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        assert!(schema_service.db_service.get_connection().lock().is_ok());
    }

    #[test]
    fn test_create_fresh_schema() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        let connection = schema_service.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Test creating fresh schema
        let result = schema_service.create_fresh_schema(&db);
        assert!(result.is_ok());

        // Verify tables were created
        let tables = [
            "images",
            "text_chunks",
            "jobs",
            "drives",
            "drive_mounts",
            "transcriptions",
            "watched_folders",
            "watched_folder_file_state",
            "schema_info",
            "app_installations",
            "app_settings",
            "app_logs",
            "generations",
            "vec_images",
            "vec_text_chunks",
        ];

        for table in &tables {
            let count: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Table {} should exist", table);
        }
    }

    #[test]
    fn test_check_nomic_schema() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        let connection = schema_service.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Initially should not have Nomic schema
        let has_schema = schema_service.check_nomic_schema(&db).unwrap();
        assert!(!has_schema);

        // Create schema and check again
        schema_service.create_fresh_schema(&db).unwrap();
        let has_schema = schema_service.check_nomic_schema(&db).unwrap();
        assert!(has_schema);
    }

    #[test]
    fn test_check_existing_tables() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        let connection = schema_service.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Initially should not have existing tables
        let has_tables = schema_service.check_existing_tables(&db).unwrap();
        assert!(!has_tables);

        // Create some tables and check again
        db.execute(
            "CREATE TABLE images (id TEXT PRIMARY KEY)",
            rusqlite::params![],
        )
        .unwrap();
        let has_tables = schema_service.check_existing_tables(&db).unwrap();
        assert!(has_tables);
    }

    #[test]
    fn test_jobs_table_exists() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        let connection = schema_service.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Initially should not have jobs table
        let exists = schema_service.jobs_table_exists(&db);
        assert!(!exists);

        // Create jobs table and check again
        schema_service.create_jobs_table(&db).unwrap();
        let exists = schema_service.jobs_table_exists(&db);
        assert!(exists);
    }

    #[test]
    fn test_ensure_jobs_table_exists() {
        let temp_dir = tempdir().unwrap();
        let db_service =
            DatabaseService::new_with_path(Some(temp_dir.path().to_path_buf())).unwrap();
        let schema_service = SchemaService::new(Arc::new(db_service));
        let connection = schema_service.db_service.get_connection();
        let db = connection.lock().unwrap();

        // Should create jobs table if it doesn't exist
        let result = schema_service.ensure_jobs_table_exists(&db);
        assert!(result.is_ok());

        // Should not fail if table already exists
        let result = schema_service.ensure_jobs_table_exists(&db);
        assert!(result.is_ok());
    }
}
