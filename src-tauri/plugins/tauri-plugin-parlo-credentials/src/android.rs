//! The Rust handle to the Kotlin `CredentialsPlugin`. Calls block until
//! Kotlin answers, so they must not run on Android's main thread.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use tauri::Runtime;
use tauri::plugin::{PluginApi, PluginHandle};

const PLUGIN_IDENTIFIER: &str = "com.ryanbordin.parloplayer.credentials";

/// The error message comes from Kotlin and never contains the password.
#[derive(Debug, thiserror::Error)]
#[error("Android Keystore: {0}")]
pub struct Error(#[from] tauri::plugin::mobile::PluginInvokeError);

pub struct CredentialStore<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Clone for CredentialStore<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(api: PluginApi<R, C>) -> Result<CredentialStore<R>, Error> {
    Ok(CredentialStore(api.register_android_plugin(PLUGIN_IDENTIFIER, "CredentialsPlugin")?))
}

#[derive(Deserialize)]
struct Stored {
    #[serde(default)]
    password: Option<String>,
}

impl<R: Runtime> CredentialStore<R> {
    /// `None` when nothing is saved for `account`, or when it can no longer be
    /// read (the Keystore key was replaced): the app then asks to log in.
    pub fn get(&self, account: &str) -> Result<Option<String>, Error> {
        let stored: Stored = self.0.run_mobile_plugin("getPassword", json!({ "account": account }))?;
        Ok(stored.password)
    }

    pub fn set(&self, account: &str, password: &str) -> Result<(), Error> {
        self.run("setPassword", json!({ "account": account, "password": password }))
    }

    /// Succeeds when nothing is saved.
    pub fn delete(&self, account: &str) -> Result<(), Error> {
        self.run("deletePassword", json!({ "account": account }))
    }

    fn run(&self, command: &str, payload: serde_json::Value) -> Result<(), Error> {
        self.0.run_mobile_plugin::<serde_json::Value>(command, payload)?;
        Ok(())
    }
}
