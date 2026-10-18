use std::sync::Arc;

use tauri::{Manager, RunEvent};

use playback::Playback;
use state::{AppState, PlaybackSlot};

mod commands;
mod db;
mod playback;
mod player;
mod session;
mod state;
pub mod subsonic;
mod sync;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(AppState::default()))
        .setup(|app| {
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
            commands::play_context,
            commands::restore_queue,
            commands::next,
            commands::previous,
            commands::play_queue_index,
            commands::set_shuffle,
            commands::cycle_repeat,
            commands::get_queue,
            commands::toggle_pause,
            commands::seek,
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
