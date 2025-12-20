mod engine; 

use pdfium_render::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdfium = engine::init_pdfium()?;

    let document = pdfium.load_pdf_from_file("test/form-test.pdf", None)?;

    document
        .pages()
        .iter()
        .enumerate()
        .for_each(|(index, page)| {
            println!("=============== Page {} ==============", index);
            
            let text = page.text().unwrap().all(); 
            println!("{}", text);
        });

    Ok(())
}}
