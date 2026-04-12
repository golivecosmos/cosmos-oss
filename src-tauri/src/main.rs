use tauri::{
    generate_context,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};

mod commands;
mod constants;
mod ffmpeg_thumbnail;
mod models;
mod services;
mod thumbnail;
mod thumbnail_cache;
mod utils;

use services::startup::{AppState, StartupManager};

use commands::{
    add_watched_folder,
    bulk_job_operations,
    cancel_download,
    check_models_status,
    check_search_status,
    clean_stale_entries,
    clear_and_redownload_models,
    clear_search_index,
    consume_full_app_handoff,
    copy_to_clipboard,
    create_error_report,
    delete_drive_from_database,
    delete_generation,
    download_models,
    edit_video,
    ensure_quick_window,
    // File operations
    file_exists,
    focus_quick_search_input,
    generate_video_prompt,
    get_all_drives,
    get_all_drives_with_metadata,
    get_all_generations,
    get_app_by_id,
    get_app_state_info,
    // Configuration commands
    get_config_info,
    // Drive management
    get_connected_drives,
    get_desktop_path, // App installation commands
    // Clustering
    compute_clusters,
    get_cluster_files,
    get_clusters,
    get_file_positions,
    get_drive_for_path,
    get_drive_indexed_files,
    get_drive_info,
    get_drive_metadata,
    get_drive_stats,
    get_file_metadata,
    get_generated_json_prompt,
    get_generation_by_id,
    get_generation_stats,
    get_indexed_count,
    get_indexed_directory,
    get_indexed_files,
    get_indexed_files_grouped,
    get_indexed_files_grouped_paginated,
    get_installed_apps,
    get_jobs,
    get_log_file_path,
    get_migration_info,
    get_recent_logs,
    get_system_info,
    get_transcription_by_path,
    get_video_generation_status,
    generate_briefing,
    get_gemma4_status,
    describe_file,
    get_file_description,
    is_gemma4_downloaded,
    download_gemma4_model,
    hide_quick_panel,
    index_directory,
    index_file,
    scan_directory,
    index_image,
    index_video,
    install_app,
    is_directory,
    // System
    is_ffmpeg_available,
    is_path_on_external_drive,
    is_video_in_studio,
    is_whisper_model_available,
    list_directory,
    list_directory_contents,
    list_directory_paginated,
    list_directory_recursive,
    list_watched_folders,
    manage_job_queue,
    open_full_app,
    open_full_app_internal,
    open_with_default_app,
    package_logs_for_support,
    read_file_as_base64,
    read_file_content,
    read_file_preview,
    refresh_drives,
    reload_models,
    remove_watched_folder,
    retry_job,
    search_semantic,
    search_visual,
    send_video_to_studio,
    set_custom_db_path,
    set_indexed_directory,
    set_queue_processing,
    set_watched_folder_enabled,
    show_in_file_manager,
    show_quick_panel,
    stop_and_clear_queue,
    cancel_jobs_by_folder,
    sync_drives_to_database,
    toggle_quick_panel,
    toggle_quick_panel_internal,
    // Audio commands
    transcribe_audio_file,
    // Transcription
    transcribe_file,
    trigger_watched_folder_backfill,
    trim_video,
    uninstall_app,
    update_drive_metadata,
    update_drive_status,
    validate_audio_file,
    FullAppHandoffState,
    WindowMetricsState,
    MAIN_WINDOW_LABEL,
    QUICK_WINDOW_LABEL,
};

use thumbnail::generate_video_thumbnail;

// Debug commands - only imported in debug builds
#[cfg(debug_assertions)]
use commands::{
    debug_model_status, get_database_schema_info, get_sqlite_stats, recreate_sqlite_virtual_table,
};

const QUICK_SHORTCUT: &str = "CommandOrControl+Shift+Space";
const TRAY_MENU_OPEN_QUICK: &str = "tray_open_quick";
const TRAY_MENU_OPEN_FULL: &str = "tray_open_full";
const TRAY_MENU_QUIT: &str = "tray_quit";

fn setup_tray(app: &tauri::App) -> Result<(), String> {
    let open_quick = MenuItem::with_id(
        app,
        TRAY_MENU_OPEN_QUICK,
        "Open Quick Panel",
        true,
        None::<&str>,
    )
    .map_err(|e| format!("Failed to create tray menu item (quick): {}", e))?;
    let open_full = MenuItem::with_id(
        app,
        TRAY_MENU_OPEN_FULL,
        "Open Full App",
        true,
        None::<&str>,
    )
    .map_err(|e| format!("Failed to create tray menu item (full): {}", e))?;
    let quit = MenuItem::with_id(app, TRAY_MENU_QUIT, "Quit", true, None::<&str>)
        .map_err(|e| format!("Failed to create tray menu item (quit): {}", e))?;

    let menu = Menu::with_items(app, &[&open_quick, &open_full, &quit])
        .map_err(|e| format!("Failed to create tray menu: {}", e))?;

    let mut tray_builder = TrayIconBuilder::with_id("cosmos_tray")
        .menu(&menu)
        .tooltip("Cosmos")
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            TRAY_MENU_OPEN_QUICK => {
                let _ = toggle_quick_panel_internal(app_handle);
            }
            TRAY_MENU_OPEN_FULL => {
                let _ = open_full_app_internal(app_handle, None);
            }
            TRAY_MENU_QUIT => {
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    let _ = toggle_quick_panel_internal(tray.app_handle());
                }
            }
        });

    if let Some(default_icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(default_icon.clone().into());
    }

    tray_builder
        .build(app)
        .map_err(|e| format!("Failed to build tray icon: {}", e))?;

    Ok(())
}

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
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == QUICK_WINDOW_LABEL || window.label() == MAIN_WINDOW_LABEL {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = toggle_quick_panel_internal(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([QUICK_SHORTCUT])
                .expect("Failed to register global shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = toggle_quick_panel_internal(app);
                    }
                })
                .build(),
        )
        .manage(app_state)
        .manage(FullAppHandoffState::default())
        .manage(WindowMetricsState::default())
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            if let Err(e) = ensure_quick_window(&app.handle()) {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }

            if let Err(e) = setup_tray(app) {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }

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
            read_file_preview,
            read_file_as_base64,
            index_image,
            index_file,
            index_video,
            index_directory,
            scan_directory,
            generate_briefing,
            get_gemma4_status,
            describe_file,
            get_file_description,
            is_gemma4_downloaded,
            download_gemma4_model,
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
            list_directory_paginated,
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
            cancel_jobs_by_folder,
            add_watched_folder,
            remove_watched_folder,
            list_watched_folders,
            set_watched_folder_enabled,
            trigger_watched_folder_backfill,
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
            generate_video_prompt,
            get_video_generation_status,
            get_generated_json_prompt,
            get_all_generations,
            get_generation_by_id,
            delete_generation,
            get_generation_stats,
            send_video_to_studio,
            is_video_in_studio,
            trim_video,
            edit_video,
            get_desktop_path,
            install_app,
            get_installed_apps,
            get_app_by_id,
            show_quick_panel,
            hide_quick_panel,
            toggle_quick_panel,
            open_full_app,
            focus_quick_search_input,
            consume_full_app_handoff,
            uninstall_app,
            // Clustering
            compute_clusters,
            get_clusters,
            get_file_positions,
            get_cluster_files,
        ]);
    }

    // Add production commands only in release builds
    #[cfg(not(debug_assertions))]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            list_directory_contents,
            get_file_metadata,
            read_file_content,
            read_file_preview,
            read_file_as_base64,
            index_image,
            index_file,
            index_video,
            index_directory,
            scan_directory,
            generate_briefing,
            get_gemma4_status,
            describe_file,
            get_file_description,
            is_gemma4_downloaded,
            download_gemma4_model,
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
            list_directory_paginated,
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
            cancel_jobs_by_folder,
            add_watched_folder,
            remove_watched_folder,
            list_watched_folders,
            set_watched_folder_enabled,
            trigger_watched_folder_backfill,
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
            generate_video_prompt,
            get_video_generation_status,
            get_generated_json_prompt,
            get_all_generations,
            get_generation_by_id,
            delete_generation,
            get_generation_stats,
            send_video_to_studio,
            is_video_in_studio,
            trim_video,
            edit_video,
            get_desktop_path,
            install_app,
            get_installed_apps,
            get_app_by_id,
            show_quick_panel,
            hide_quick_panel,
            toggle_quick_panel,
            open_full_app,
            focus_quick_search_input,
            consume_full_app_handoff,
            uninstall_app,
            // Clustering
            compute_clusters,
            get_clusters,
            get_file_positions,
            get_cluster_files,
        ]);
    }

    builder
        .run(generate_context!())
        .expect("error while running tauri application");
}
