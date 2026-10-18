use std::sync::Arc;

use tauri::Manager;

use player::Player;
use state::{AppState, PlayerSlot};

mod commands;
mod db;
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
            let handle = app.handle().clone();
            let player = Player::new(&[], move |event| commands::emit_player_event(&handle, event))
                .map_err(|e| e.to_string());
            app.manage(PlayerSlot(player));
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
            commands::play_song,
            commands::toggle_pause,
            commands::seek,
            commands::set_volume,
            commands::get_player_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
