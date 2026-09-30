//! Build script: on Windows targets, compile the branding icon into the
//! executable, so Explorer, shortcuts and a pinned taskbar button show it even
//! while the game is closed (the live window's icon is set at run time, see
//! `src/icon.rs`). Other targets are left alone. A build script runs on the
//! host, so the target comes from Cargo's environment, never from `cfg!`.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/branding/whistle.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/branding/whistle.ico")
            .compile()
            .expect(
                "could not compile the Windows icon resource (needs rc.exe from the Windows SDK, or windres for GNU)",
            );
    }
}
