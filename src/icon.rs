//! The window's icon: the branding kit's hat on the live window's title bar,
//! and on Windows its taskbar button and Alt-Tab (`ICON_BIG`). The
//! executable's own icon (Explorer, shortcuts, a pinned button while the game
//! is closed) is compiled in by `build.rs`; both use the same art
//! (`assets/branding/`, see `assets/SOURCES.md`).
//!
//! The PNG is embedded, so the icon needs no asset folder and works from any
//! working directory. `--headless` never builds a Bevy app (`app::run`
//! returns first), so nothing here runs there.

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;

/// The 256 px RGBA export: Windows' ceiling for `ICON_BIG`.
const ICON_PNG: &[u8] = include_bytes!("../assets/branding/whistle-icon-256.png");

/// The icon as straight (not premultiplied) RGBA8 rows, with its width and
/// height.
pub fn rgba() -> Result<(Vec<u8>, u32, u32), String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
    let image = Image::from_buffer(
        ICON_PNG,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::MAIN_WORLD,
    )
    .map_err(|e| e.to_string())?;
    let rgba = image.try_into_dynamic().map_err(|e| e.to_string())?.into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

/// Puts the icon on the primary window's native window, once per native
/// window: on a later frame if it does not exist yet, and again if it is
/// ever made anew. Main thread only (`NonSendMarker`): winit's windows live
/// there.
pub fn apply(window: Query<Entity, With<PrimaryWindow>>, mut done: Local<Option<u64>>, _main: NonSendMarker) {
    let Ok(entity) = window.single() else {
        return;
    };
    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(native) = windows.get_window(entity) else {
            return;
        };
        let id = u64::from(native.id());
        if *done == Some(id) {
            return;
        }
        *done = Some(id);
        let icon =
            rgba().and_then(|(pixels, w, h)| winit::window::Icon::from_rgba(pixels, w, h).map_err(|e| e.to_string()));
        match icon {
            Ok(icon) => {
                #[cfg(target_os = "windows")]
                {
                    use winit::platform::windows::WindowExtWindows;
                    native.set_taskbar_icon(Some(icon.clone()));
                }
                native.set_window_icon(Some(icon));
            }
            Err(e) => warn!("the window icon could not be made: {e}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_icon_is_a_256_px_window_icon_with_its_transparency() {
        let (pixels, w, h) = rgba().expect("the embedded PNG decodes");
        assert_eq!((w, h), (256, 256), "Windows' ICON_BIG ceiling");
        assert_eq!(pixels.len(), (w * h * 4) as usize, "RGBA8");
        let alpha = |a: u8| pixels.chunks_exact(4).any(|p| p[3] == a);
        assert!(alpha(0) && alpha(255), "clear corners and a solid hat");
        assert!(winit::window::Icon::from_rgba(pixels, w, h).is_ok(), "winit takes it");
    }
}
