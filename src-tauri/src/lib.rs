use std::sync::Arc;

use tauri::{Manager, RunEvent};

use covers::{CoverCache, Covers};
use playback::Playback;
use state::{AppState, PlaybackSlot};

mod commands;
mod covers;
mod db;
mod home;
mod locale;
mod now_playing;
mod playback;
mod player;
mod session;
mod state;
pub mod subsonic;
mod sync;
mod test_library;
mod time;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(locale::plugin())
        .manage(Arc::new(AppState::default()));
    covers::register(builder)
        .setup(|app| {
            // Spec 12.4: covers on disk, 500 MB at most.
            let dir = app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("covers");
            app.manage(Arc::new(Covers::new(CoverCache::new(dir, covers::DEFAULT_LIMIT_BYTES))));

            let playback = Playback::start(app.handle().clone()).map_err(|e| e.to_string());
            app.manage(PlaybackSlot(playback));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::test_connection,
            commands::get_session,
            commands::login,
            commands::logout,
            commands::get_library_status,
            commands::sync_library,
            commands::list_songs,
            commands::list_playlists,
            commands::get_home_library,
            commands::get_home_activity,
            commands::get_home_section,
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
            commands::set_album_starred,
            commands::get_album_page,
            commands::get_playlist_page,
            commands::play_shuffled,
            commands::list_albums,
            commands::list_song_ids,
            commands::get_song_rows,
            commands::play_mix,
            commands::is_test_library,
            commands::play_albums,
            commands::move_in_queue,
            commands::remove_from_queue,
            commands::create_playlist,
            commands::get_lyrics,
            commands::get_related,
            commands::toggle_pause,
            commands::seek,
            commands::seek_by,
            commands::toggle_mute,
            commands::set_volume,
            commands::get_player_state,
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
