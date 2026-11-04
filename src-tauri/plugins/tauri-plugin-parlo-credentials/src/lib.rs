//! Saved passwords on Android (spec 12.5: the password never goes to disk
//! in the clear).
//!
//! The Kotlin side (`android/`) keeps an AES key in the Android Keystore,
//! which never leaves the system's secure store, and saves only the
//! encrypted password in the app's private storage. This crate forwards the
//! calls; the app uses it where desktop uses the OS credential store.
//!
//! On other platforms the plugin registers nothing.

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::{CredentialStore, Error};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("parlo-credentials")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                _app.manage(android::init(_api)?);
            }
            Ok(())
        })
        .build()
}
