//! App preferences (spec 6.10). The settings themselves are shared with the
//! Android app (`parlo_core::settings`); the window scale is desktop only.

use tauri::{LogicalSize, Runtime, WebviewWindow};

pub use parlo_core::settings::*;

/// Narrowest window at 100% (`minWidth` in tauri.conf.json), in logical pixels.
const BASE_MIN_WIDTH: f64 = 1000.0;

/// Interface scale (spec 6.10): the window's zoom, and its minimum width
/// grown in proportion, so the layout always has the room it needs.
pub fn apply_scale<R: Runtime>(window: &WebviewWindow<R>, scale: u16) -> tauri::Result<()> {
    let factor = f64::from(scale) / 100.0;
    window.set_zoom(factor)?;

    let mut min_width = BASE_MIN_WIDTH * factor;
    // Never wider than the screen.
    if let Some(monitor) = window.current_monitor()? {
        let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
        min_width = min_width.min(screen.width);
    }
    window.set_min_size(Some(LogicalSize::new(min_width, 0.0)))?;

    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    if size.width < min_width {
        window.set_size(LogicalSize::new(min_width, size.height))?;
    }
    Ok(())
}
