mod engine;

use clap::{Arg, Parser};
use iced::wgpu::wgc::command;
use iced::widget::{column, container, image, text};
use iced::{Element, Length};
use pdfium_render::prelude::*;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    file_name: String,
}

pub fn main() -> iced::Result {
    let args = Args::parse();

    iced::application(
        move || App::new(args.file_name.clone()),
        App::update,
        App::view,
    )
    .title("Rufium")
    .run()
}

struct App {
    document: Option<PdfDocument<'static>>,
    current_image: Option<image::Handle>,
    file_name: String,
}

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new(file_name: String) -> (Self, iced::Task<Message>) {
        let pdfium = match engine::init_pdfium() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Fatal Error loading PDFium: {}", e);
                return (
                    Self {
                        document: None,
                        current_image: None,
                        file_name,
                    },
                    iced::Task::none(),
                );
            }
        };

        let pdfium_static = Box::leak(Box::new(pdfium));

        let document = match pdfium_static.load_pdf_from_file(&file_name, None) {
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("Could not open file: {}", e);
                return (
                    Self {
                        document: None,
                        current_image: None,
                        file_name,
                    },
                    iced::Task::none(),
                );
            }
        };

        let handle = render_page_to_image(&document, 0);

        (
            Self {
                document: Some(document),
                current_image: Some(handle),
                file_name,
            },
            iced::Task::none(),
        )
    }

    fn update(&mut self, _message: Message) -> iced::Task<Message> {
        iced::Task::none()
    }

    fn view(&self) -> Element<Message> {
        let content: Element<Message> = if let Some(handle) = &self.current_image {
            image(handle.clone())
                .width(Length::Fill)
                .content_fit(iced::ContentFit::Contain)
                .into()
        } else {
            text("Could not load PDF. Check logs.").size(30).into()
        };

        container(column![content])
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

fn render_page_to_image(document: &PdfDocument, page_index: u16) -> image::Handle {
    let page = document.pages().get(page_index).unwrap();

    let render_config = PdfRenderConfig::new()
        .set_target_width(2480)
        .set_maximum_height(3508)
        .rotate_if_landscape(PdfPageRenderRotation::None, true);

    let bitmap = page.render_with_config(&render_config).unwrap();

    let image = bitmap.as_image();
    let rgba = image.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let pixels = rgba.into_raw();

    image::Handle::from_rgba(width, height, pixels)
}
