use std::sync::Arc;

use tauri::{Manager, RunEvent};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use covers::{CoverCache, Covers};
use downloads::Saves;
use playback::Playback;
use state::{AppState, PlaybackSlot};

mod artist_page;
mod auto_sync;
mod commands;
mod covers;
mod db;
mod downloads;
mod home;
mod locale;
mod navidrome;
mod now_playing;
mod playback;
mod player;
mod playlists;
mod session;
mod settings;
mod state;
pub mod subsonic;
mod sync;
mod test_library;
mod time;

/// Each log file grows to this size, then a new one starts.
const LOG_FILE_BYTES: u128 = 2 * 1024 * 1024;
/// Log files kept, the newest ones.
const LOG_FILES_KEPT: usize = 5;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // Spec 6.10 (About > Open logs folder): the terminal and a file in the
        // app's log folder. Messages never carry passwords, tokens or URLs
        // with credentials (spec 12.5).
        .plugin(
            tauri_plugin_log::Builder::new()
                .clear_targets()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: Some("parlo".into()) }),
                ])
                .level(log::LevelFilter::Info)
                .max_file_size(LOG_FILE_BYTES)
                .rotation_strategy(RotationStrategy::KeepSome(LOG_FILES_KEPT))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        // "Save to computer" (spec 8.2): the system's file dialogs, opened by the backend.
        .plugin(tauri_plugin_dialog::init())
        .plugin(locale::plugin())
        .manage(Arc::new(AppState::default()));
    covers::register(builder)
        .setup(|app| {
            // Spec 6.10: the preferences first, so the cover cache, the player
            // and the page start with them.
            if let Ok(config_dir) = app.path().app_config_dir() {
                app.state::<Arc<AppState>>().set_settings(settings::load(&config_dir));
            }

            // Spec 12.4: covers on disk, with the limit of the settings.
            let dir = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("covers");
            let limit = app.state::<Arc<AppState>>().settings().cover_cache_bytes();
            app.manage(Arc::new(Covers::new(CoverCache::new(dir, limit))));
            if let Some(window) = app.get_webview_window("main") {
                let scale = app.state::<Arc<AppState>>().settings().scale;
                if let Err(error) = settings::apply_scale(&window, scale) {
                    log::warn!("setup: could not apply the interface scale: {error}");
                }
            }

            app.manage(Saves::new(app.handle().clone()));
            let playback = Playback::start(app.handle().clone()).map_err(|e| e.to_string());
            app.manage(PlaybackSlot(playback));
            auto_sync::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_connection,
            commands::get_session,
            commands::login,
            commands::logout,
            commands::get_library_status,
            commands::sync_library,
            commands::sync_light,
            commands::list_songs,
            commands::list_playlists,
            commands::get_home_library,
            commands::get_home_activity,
            commands::get_home_section,
            commands::get_favorites,
            commands::play_album,
            commands::play_context,
            commands::restore_queue,
            commands::next,
            commands::previous,
            commands::play_queue_index,
            commands::set_shuffle,
            commands::cycle_repeat,
            commands::get_queue,
            commands::set_song_starred,
            commands::set_songs_starred,
            commands::set_album_starred,
            commands::get_album_page,
            commands::get_playlist_page,
            commands::list_albums,
            commands::list_song_ids,
            commands::get_song_rows,
            commands::search,
            commands::get_recent_searches,
            commands::add_recent_search,
            commands::remove_recent_search,
            commands::play_mix,
            commands::is_test_library,
            commands::play_albums,
            commands::list_artists,
            commands::play_artists,
            commands::get_artist_page,
            commands::get_artist_about,
            commands::set_artist_starred,
            commands::move_in_queue,
            commands::remove_from_queue,
            commands::create_playlist,
            commands::play_item,
            commands::enqueue,
            commands::play_similar_songs,
            commands::play_artist_radio,
            commands::add_to_playlist,
            commands::update_playlist,
            commands::remove_from_playlist,
            commands::delete_playlist,
            commands::get_song_info,
            commands::get_album_info,
            commands::get_artist_info,
            commands::save_to_computer,
            commands::cancel_saves,
            commands::open_save_folder,
            commands::is_navidrome,
            commands::open_in_navidrome,
            commands::get_lyrics,
            commands::get_related,
            commands::toggle_pause,
            commands::seek,
            commands::seek_by,
            commands::toggle_mute,
            commands::set_volume,
            commands::get_player_state,
            commands::get_app_settings,
            commands::save_app_settings,
            commands::get_audio_devices,
            commands::get_library_settings,
            commands::save_library_settings,
            commands::rebuild_library,
            commands::get_server_status,
            commands::get_sync_status,
            commands::resend_scrobbles,
            commands::get_library_summary,
            commands::get_music_folders,
            commands::clear_cover_cache,
            commands::open_link,
            commands::open_logs_folder,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Spec 5.6: save the queue one last time when the app closes.
            if let RunEvent::Exit = event
                && let Ok(playback) = &app.state::<PlaybackSlot>().0
            {
                playback.save_now();
            }
        });
}
