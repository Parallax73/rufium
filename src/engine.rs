use pdfium_render::prelude::*;
use std::error::Error;

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
