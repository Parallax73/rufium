//! PDF rendering and document handling

use pdfium_render::prelude::*;
use std::error::Error;
use std::path::Path;

/// Initialize the PDFium library
pub fn init_pdfium() -> Result<Pdfium, Box<dyn Error>> {
    let lib_name = if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else {
        "libpdfium.so"
    };

    let bindings = Pdfium::bind_to_library(format!("./{}", lib_name))
        .or_else(|_| Pdfium::bind_to_library(format!("/usr/lib/{}", lib_name)))?;

    Ok(Pdfium::new(bindings))
}

/// Represents a PDF document with metadata
pub struct PdfDocument {
    pub total_pages: u16,
}

impl PdfDocument {
    /// Load a PDF document from a file path
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn Error>> {
        let pdfium = init_pdfium()?;
        let document = pdfium.load_pdf_from_file(path, None)?;
        let total_pages = document.pages().len();
        Ok(Self { total_pages })
    }
}

/// PDF renderer that handles page rendering with optimization
pub struct PdfRenderer;

impl PdfRenderer {
    /// Render a page to RGBA pixels
    /// Returns (pixels, width, height) or None on failure
    pub fn render_page_to_pixels(
        document: &pdfium_render::prelude::PdfDocument,
        page_index: u16,
        target_w: u16,
        _target_h: u16,
    ) -> Option<(Vec<u8>, u32, u32)> {
        let page = document.pages().get(page_index).ok()?;

        let mut render_config =
            PdfRenderConfig::new().rotate_if_landscape(PdfPageRenderRotation::None, true);

        // Optimize: use at least 800px width, or the target width
        if target_w > 0 {
            render_config = render_config.set_target_width(target_w.max(800) as i32);
        } else {
            render_config = render_config.set_target_width(2000);
        }

        let bitmap = page.render_with_config(&render_config).ok()?;
        let img = bitmap.as_image();
        let rgba = img.to_rgba8();
        let width = rgba.width();
        let height = rgba.height();
        let pixels = rgba.into_raw();

        Some((pixels, width, height))
    }
}
