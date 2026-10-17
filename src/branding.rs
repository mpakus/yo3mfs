//! Approved artwork embedded in the executable; no runtime files or network access.
use eframe::egui::{self, TextureHandle};

pub const APP_ID: &str = "com.yo3mfs.viewer";
pub const ICON_PNG: &[u8] = include_bytes!("../assets/branding/icons/256.png");
const LOGO_PNG: &[u8] = include_bytes!("../assets/branding/yo3mfs-logo-dark.png");

pub struct Branding {
    pub icon: TextureHandle,
    pub logo: TextureHandle,
}

impl Branding {
    pub fn load(ctx: &egui::Context) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            icon: texture(ctx, "yo3mfs-icon", ICON_PNG)?,
            logo: texture(ctx, "yo3mfs-logo", LOGO_PNG)?,
        })
    }
}

fn texture(
    ctx: &egui::Context,
    name: &str,
    bytes: &[u8],
) -> Result<TextureHandle, Box<dyn std::error::Error + Send + Sync>> {
    let decoded = eframe::icon_data::from_png_bytes(bytes)?;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [decoded.width as usize, decoded.height as usize],
        &decoded.rgba,
    );
    Ok(ctx.load_texture(name, image, egui::TextureOptions::LINEAR))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_artwork_decodes_and_icon_preserves_transparency() {
        let icon = eframe::icon_data::from_png_bytes(ICON_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
        // The generated PNG has residual alpha of 1/255 in a few edge pixels.
        // Accept that original antialiasing, but reject an opaque background.
        for pixel in [0, 255, 256 * 255, 256 * 256 - 1] {
            assert!(icon.rgba[pixel * 4 + 3] <= 1);
        }
        assert!(icon.rgba.chunks_exact(4).any(|p| p[3] == 255));
        let brand = Branding::load(&egui::Context::default()).unwrap();
        let size = brand.logo.size();
        assert!(size[0] >= 3 * size[1], "the wordmark must stay horizontal");
    }
}
