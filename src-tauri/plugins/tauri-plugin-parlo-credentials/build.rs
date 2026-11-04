// No commands for the interface: only the app's Rust code calls the plugin.
const COMMANDS: &[&str] = &[];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).android_path("android").build();
}
