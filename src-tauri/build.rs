use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    // The build script runs on the host; check the *target* OS.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        setup_libmpv_windows();
    }
    tauri_build::build()
}

/// Links against `lib/mpv/mpv.lib` and copies `libmpv-2.dll` next to the
/// executables, where Windows looks for DLLs. See docs/libmpv-windows.md.
fn setup_libmpv_windows() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mpv_dir = manifest_dir.join("lib").join("mpv");
    let dll = mpv_dir.join("libmpv-2.dll");
    let import_lib = mpv_dir.join("mpv.lib");

    println!("cargo:rerun-if-changed={}", dll.display());
    println!("cargo:rerun-if-changed={}", import_lib.display());

    for file in [&dll, &import_lib] {
        if !file.exists() {
            panic!(
                "\n\nlibmpv not found: {} is missing.\n\
                 Follow docs/libmpv-windows.md to download libmpv and generate mpv.lib.\n\n",
                file.display()
            );
        }
    }
    println!("cargo:rustc-link-search=native={}", mpv_dir.display());

    // OUT_DIR is target/<profile>/build/<package>-<hash>/out.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("unexpected OUT_DIR layout");
    // The app runs from target/<profile>/, tests from target/<profile>/deps/.
    for dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
        copy_if_changed(&dll, &dir.join("libmpv-2.dll"));
    }
}

/// The DLL is large (~120 MB), so skip the copy when it is already there.
fn copy_if_changed(from: &Path, to: &Path) {
    let same = match (fs::metadata(from), fs::metadata(to)) {
        (Ok(a), Ok(b)) => a.len() == b.len() && a.modified().ok() <= b.modified().ok(),
        _ => false,
    };
    if !same {
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(from, to).unwrap_or_else(|e| panic!("copying {}: {e}", from.display()));
    }
}
