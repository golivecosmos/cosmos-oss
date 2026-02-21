use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

/// Represents a single database migration
pub struct Migration {
    pub up_sql: Vec<String>,    // SQL statements to apply the migration
    pub down_sql: Vec<String>,  // SQL statements to rollback the migration (optional)
}

impl Migration {
    pub fn new(_version: u32, _name: &str, _description: &str) -> Self {
        Self {
            up_sql: Vec::new(),
            down_sql: Vec::new(),
        }
    }

    /// Add an SQL statement to apply during migration
    pub fn add_sql(mut self, sql: &str) -> Self {
        self.up_sql.push(sql.to_string());
        self
    }

    /// Add a rollback SQL statement (for potential future use)
    pub fn add_rollback_sql(mut self, sql: &str) -> Self {
        self.down_sql.push(sql.to_string());
        self
    }
}

/// Database migration service
pub struct MigrationService {
    migrations: HashMap<u32, Migration>,
    current_version: u32,
}

impl MigrationService {
    pub fn new() -> Self {
        let mut service = Self {
            migrations: HashMap::new(),
            current_version: 0,
        };
        
        // Register all migrations
        service.register_migrations();
        service
    }

    /// Register all database migrations in order
    fn register_migrations(&mut self) {
        // Migration 1: Add drive_uuid and relative_path to images table
        let migration_1 = Migration::new(
            1, 
            "add_drive_columns_to_images", 
            "Add drive_uuid and relative_path columns to images table"
        )
        .add_sql("ALTER TABLE images ADD COLUMN drive_uuid TEXT")
        .add_sql("ALTER TABLE images ADD COLUMN relative_path TEXT")
        .add_sql("CREATE INDEX IF NOT EXISTS idx_images_drive_uuid ON images(drive_uuid)")
        .add_rollback_sql("ALTER TABLE images DROP COLUMN relative_path")
        .add_rollback_sql("ALTER TABLE images DROP COLUMN drive_uuid");

        self.migrations.insert(1, migration_1);

        // Migration 2: Add drive_uuid to jobs table for drive-specific indexing
        let migration_2 = Migration::new(
            2,
            "add_drive_support_to_jobs",
            "Add drive_uuid and relative_path to jobs table"
        )
        .add_sql("ALTER TABLE jobs ADD COLUMN drive_uuid TEXT")
        .add_sql("ALTER TABLE jobs ADD COLUMN relative_path TEXT")
        .add_sql("CREATE INDEX IF NOT EXISTS idx_jobs_drive_uuid ON jobs(drive_uuid)")
        .add_rollback_sql("ALTER TABLE jobs DROP COLUMN relative_path")
        .add_rollback_sql("ALTER TABLE jobs DROP COLUMN drive_uuid");

        self.migrations.insert(2, migration_2);

        // Migration 3: Add drive_uuid to transcriptions table
        let migration_3 = Migration::new(
            3,
            "add_drive_support_to_transcriptions",
            "Add drive_uuid and relative_path to transcriptions table"
        )
        .add_sql("ALTER TABLE transcriptions ADD COLUMN drive_uuid TEXT")
        .add_sql("ALTER TABLE transcriptions ADD COLUMN relative_path TEXT")
        .add_sql("CREATE INDEX IF NOT EXISTS idx_transcriptions_drive_uuid ON transcriptions(drive_uuid)")
        .add_rollback_sql("ALTER TABLE transcriptions DROP COLUMN relative_path")
        .add_rollback_sql("ALTER TABLE transcriptions DROP COLUMN drive_uuid");

        self.migrations.insert(3, migration_3);

        // Set the target version to the highest migration number
        self.current_version = 3;
    }

    /// Get the current database version
    pub fn get_current_version(&self, db: &Connection) -> Result<u32> {
        // First check if schema_migrations table exists
        let table_exists: bool = db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |row| row.get::<_, i64>(0).map(|count| count > 0),
        )?;

        if !table_exists {
            return Ok(0); // No migrations have been applied
        }

        let version = db.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, u32>(0),
        ).unwrap_or(0);

        Ok(version)
    }

    /// Check if the database needs migrations
    pub fn needs_migration(&self, db: &Connection) -> Result<bool> {
        let current_version = self.get_current_version(db)?;
        Ok(current_version < self.current_version)
    }

    /// Get migration history
    pub fn get_migration_history(&self, db: &Connection) -> Result<Vec<serde_json::Value>> {
        let table_exists: bool = db.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |row| row.get::<_, i64>(0).map(|count| count > 0),
        )?;

        if !table_exists {
            return Ok(Vec::new());
        }

        let mut stmt = db.prepare(
            "SELECT version, name, description, applied_at, execution_time_ms 
             FROM schema_migrations 
             ORDER BY version ASC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "version": row.get::<_, u32>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "applied_at": row.get::<_, String>(3)?,
                "execution_time_ms": row.get::<_, i64>(4)?
            }))
        })?;

        let history: Result<Vec<_>, _> = rows.collect();
        Ok(history?)
    }
}

/// Get migration status information
pub fn get_migration_info(db: &Connection) -> Result<serde_json::Value> {
    let migration_service = MigrationService::new();
    let current_version = migration_service.get_current_version(db)?;
    let needs_migration = migration_service.needs_migration(db)?;
    let history = migration_service.get_migration_history(db)?;
    
    Ok(serde_json::json!({
        "current_version": current_version,
        "target_version": migration_service.current_version,
        "needs_migration": needs_migration,
        "history": history
    }))
}