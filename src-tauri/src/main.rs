use tauri::generate_context;

mod constants;
mod models;
mod services;
mod utils;
mod commands;
mod thumbnail;
mod ffmpeg_thumbnail;
mod thumbnail_cache;


use services::startup::{StartupManager, AppState};

use commands::{
    // Audio commands
    transcribe_audio_file, validate_audio_file,
    get_transcription_by_path, is_whisper_model_available,
    // File operations
    file_exists, is_directory, get_log_file_path,
    list_directory_contents, get_file_metadata, read_file_content, read_file_as_base64,
    list_directory, list_directory_recursive,
    copy_to_clipboard, show_in_file_manager, open_with_default_app,
    create_error_report, get_recent_logs, package_logs_for_support,
    search_semantic, search_visual, check_search_status,
    check_models_status, clear_and_redownload_models, download_models, reload_models,
    index_image, index_file, index_video, index_directory,
    // Transcription
    transcribe_file,
    get_indexed_files, get_indexed_directory, set_indexed_directory, get_indexed_files_grouped, get_indexed_files_grouped_paginated, get_indexed_count, clean_stale_entries,
    // System
    is_ffmpeg_available, get_system_info, get_app_state_info, cancel_download,
    clear_search_index,
    get_jobs, manage_job_queue, retry_job, bulk_job_operations, set_queue_processing, stop_and_clear_queue,
    // Drive management
    get_connected_drives, get_drive_info, refresh_drives, get_all_drives, update_drive_status,
    get_drive_for_path, is_path_on_external_drive, get_drive_indexed_files, get_drive_stats,
    update_drive_metadata, get_all_drives_with_metadata, get_drive_metadata, sync_drives_to_database,
    delete_drive_from_database,
    get_migration_info,
    // Configuration commands
    get_config_info, set_custom_db_path,
    generate_video_prompt, get_video_generation_status, get_generated_json_prompt,
    get_all_generations, get_generation_by_id, delete_generation, get_generation_stats,
    send_video_to_studio, is_video_in_studio, trim_video, edit_video,
    get_desktop_path,    // App installation commands
    install_app, get_installed_apps, get_app_by_id, uninstall_app,
};

use thumbnail::generate_video_thumbnail;

// Debug commands - only imported in debug builds
#[cfg(debug_assertions)]
use commands::{
    debug_model_status, get_sqlite_stats, recreate_sqlite_virtual_table, get_database_schema_info,
};

#[tokio::main]
async fn main() {
    env_logger::init();

    // Initialize startup manager and services
    let mut startup_manager = StartupManager::new();
    let app_state = match startup_manager.initialize_services().await {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to initialize application: {}", e);
            return;
        }
    };

    // Launch Tauri application
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            // Setup background tasks using the startup manager
            if let Err(e) = startup_manager.setup_background_tasks(app) {
                eprintln!("Failed to setup background tasks: {}", e);
            }
            Ok(())
        });

    // Add debug commands only in debug builds
    #[cfg(debug_assertions)]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            list_directory_contents,
            get_file_metadata,
            read_file_content,
            read_file_as_base64,
            index_image,
            index_file,
            index_video,
            index_directory,
            transcribe_file,
            transcribe_audio_file,
            validate_audio_file,
            get_transcription_by_path,
            is_whisper_model_available,
            is_ffmpeg_available,
            get_indexed_files,
            get_indexed_files_grouped,
            get_indexed_files_grouped_paginated,
            get_indexed_count,
            get_indexed_directory,
            set_indexed_directory,
            list_directory,
            list_directory_recursive,
            file_exists,
            is_directory,
            get_log_file_path,
            copy_to_clipboard,
            show_in_file_manager,
            open_with_default_app,
            create_error_report,
            get_recent_logs,
            package_logs_for_support,
            search_semantic,
            search_visual,
            check_search_status,
            check_models_status,
            clear_and_redownload_models,
            download_models,
            reload_models,
            clear_search_index,
            cancel_download,
            get_system_info,
            get_app_state_info,
            clean_stale_entries,
            // Debug commands - only available in debug builds
            debug_model_status,
            get_sqlite_stats,
            recreate_sqlite_virtual_table,
            get_database_schema_info,
            get_jobs,
            manage_job_queue,
            retry_job,
            bulk_job_operations,
            set_queue_processing,
            stop_and_clear_queue,
            get_connected_drives,
            get_drive_info,
            refresh_drives,
            get_all_drives,
            update_drive_status,
            get_drive_for_path,
            is_path_on_external_drive,
            get_drive_indexed_files,
            get_drive_stats,
            update_drive_metadata,
            get_all_drives_with_metadata,
            get_drive_metadata,
            sync_drives_to_database,
            delete_drive_from_database,
            get_migration_info,
            generate_video_thumbnail,
            get_config_info,
            set_custom_db_path,
            generate_video_prompt, get_video_generation_status, get_generated_json_prompt,
            get_all_generations, get_generation_by_id, delete_generation, get_generation_stats,
            send_video_to_studio, is_video_in_studio, trim_video, edit_video,
            get_desktop_path,
            install_app, get_installed_apps, get_app_by_id, uninstall_app,
        ]);
    }

    // Add production commands only in release builds
    #[cfg(not(debug_assertions))]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            list_directory_contents,
            get_file_metadata,
            read_file_content,
            read_file_as_base64,
            index_image,
            index_file,
            index_video,
            index_directory,
            transcribe_file,
            transcribe_audio_file,
            validate_audio_file,
            get_transcription_by_path,
            is_whisper_model_available,
            is_ffmpeg_available,
            get_indexed_files,
            get_indexed_files_grouped,
            get_indexed_files_grouped_paginated,
            get_indexed_count,
            get_indexed_directory,
            set_indexed_directory,
            list_directory,
            list_directory_recursive,
            file_exists,
            is_directory,
            get_log_file_path,
            copy_to_clipboard,
            show_in_file_manager,
            open_with_default_app,
            create_error_report,
            get_recent_logs,
            package_logs_for_support,
            search_semantic,
            search_visual,
            check_search_status,
            check_models_status,
            clear_and_redownload_models,
            download_models,
            reload_models,
            clear_search_index,
            cancel_download,
            get_system_info,
            get_app_state_info,
            clean_stale_entries,
            get_jobs,
            manage_job_queue,
            retry_job,
            bulk_job_operations,
            set_queue_processing,
            stop_and_clear_queue,
            get_connected_drives,
            get_drive_info,
            refresh_drives,
            get_all_drives,
            update_drive_status,
            get_drive_for_path,
            is_path_on_external_drive,
            get_drive_indexed_files,
            get_drive_stats,
            update_drive_metadata,
            get_all_drives_with_metadata,
            get_drive_metadata,
            sync_drives_to_database,
            delete_drive_from_database,
            get_migration_info,
            generate_video_thumbnail,
            get_config_info,
            set_custom_db_path,
            generate_video_prompt, get_video_generation_status, get_generated_json_prompt,
            get_all_generations, get_generation_by_id, delete_generation, get_generation_stats,
            send_video_to_studio, is_video_in_studio, trim_video, edit_video,
            get_desktop_path,
            install_app, get_installed_apps, get_app_by_id, uninstall_app,
        ]);
    }

    builder
        .run(generate_context!())
        .expect("error while running tauri application");
}
