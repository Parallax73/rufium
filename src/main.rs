use clap::Parser;
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key, Modifiers};
use iced::widget::{column, container, image, text};
use iced::{time, window, Element, Event, Length, Size, Subscription, Task};
use pdfium_render::prelude::*;
use std::path::Path;
use std::process;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

mod engine;

#[derive(Parser, Debug, Clone)]
#[command(version)]
struct Args {
    #[arg(short = 'f', long)]
    file_name: String,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    EventOccurred(Event),
    WindowEvent(window::Id, window::Event),
}

enum RenderCommand {
    RenderPage(usize, u16, u16),
}

struct App {
    _file_name: String,
    current_image: Option<image::Handle>,
    current_page_index: usize,
    total_pages: u16,
    render_tx: mpsc::Sender<RenderCommand>,
    render_rx: Arc<Mutex<mpsc::Receiver<(Option<Vec<u8>>, u32, u32)>>>,
    _search_index: Arc<Mutex<Vec<String>>>,
    window_size: Size,
    window_id: Option<window::Id>,
}

pub fn main() -> iced::Result {
    let args = Args::parse();
    let path = Path::new(&args.file_name);

    if path.extension().and_then(|e| e.to_str()) != Some("pdf") {
        eprintln!("The file you entered isn't a PDF.");
        process::exit(1);
    }

    iced::application(
        move || App::new(args.file_name.clone()),
        App::update,
        App::view,
    )
    .title("Rufium")
    .subscription(App::subscription)
    .run()
}

impl App {
    fn new(file_name: String) -> (Self, Task<Message>) {
        let file_name_for_render = file_name.clone();
        let file_name_for_index = file_name.clone();

        let total_pages = {
            let pdfium = match engine::init_pdfium() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Fatal Error loading PDFium: {}", e);
                    process::exit(1);
                }
            };
            let document = match pdfium.load_pdf_from_file(&file_name, None) {
                Ok(doc) => doc,
                Err(e) => {
                    eprintln!("Could not open file: {}", e);
                    process::exit(1);
                }
            };
            document.pages().len()
        };

        let search_index = Arc::new(Mutex::new(Vec::new()));

        let (render_tx, render_thread_rx) = mpsc::channel::<RenderCommand>();
        let (ui_tx, ui_rx) = mpsc::channel::<(Option<Vec<u8>>, u32, u32)>();

        let index_store = search_index.clone();
        thread::spawn(move || {
            let pdfium = match engine::init_pdfium() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Indexer: Failed to init pdfium: {}", e);
                    return;
                }
            };
            let document = match pdfium.load_pdf_from_file(&file_name_for_index, None) {
                Ok(doc) => doc,
                Err(e) => {
                    eprintln!("Indexer: Failed to load document: {}", e);
                    return;
                }
            };

            let page_count = document.pages().len();
            for i in 0..page_count {
                if let Ok(page) = document.pages().get(i) {
                    if let Ok(text_page) = page.text() {
                        let text_content = text_page.all();
                        index_store.lock().unwrap().push(text_content);
                    }
                }
                thread::sleep(Duration::from_millis(5));
            }
        });

        thread::spawn(move || {
            let pdfium = match engine::init_pdfium() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Renderer: Failed to init pdfium: {}", e);
                    return;
                }
            };
            let document = match pdfium.load_pdf_from_file(&file_name_for_render, None) {
                Ok(doc) => doc,
                Err(e) => {
                    eprintln!("Renderer: Failed to load document: {}", e);
                    return;
                }
            };

            while let Ok(cmd) = render_thread_rx.recv() {
                match cmd {
                    RenderCommand::RenderPage(idx, w, h) => {
                        let result = render_page_to_pixels(&document, idx as u16, w, h);
                        let _ = ui_tx.send(result);
                    }
                }
            }
        });

        render_tx.send(RenderCommand::RenderPage(0, 800, 600)).unwrap();

        (
            Self {
                _file_name: file_name,
                current_image: None,
                current_page_index: 0,
                total_pages,
                render_tx,
                render_rx: Arc::new(Mutex::new(ui_rx)),
                _search_index: search_index,
                window_size: Size::new(800.0, 600.0),
                window_id: None,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if let Ok(rx) = self.render_rx.lock() {
                    if let Ok((Some(pixels), width, height)) = rx.try_recv() {
                        self.current_image =
                            Some(image::Handle::from_rgba(width, height, pixels));

                        let aspect_ratio = width as f32 / height as f32;
                        let new_height = 800.0;
                        let new_width = new_height * aspect_ratio;

                        if let Some(id) = self.window_id {
                             return window::resize(id, Size::new(new_width, new_height));
                        }
                    }
                }
                Task::none()
            }
            Message::EventOccurred(event) => {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) = event
                {
                    return self.handle_key_press(key, modifiers);
                }
                Task::none()
            }
            Message::WindowEvent(id, event) => {
                self.window_id = Some(id);
                if let window::Event::Resized(size) = event {
                    self.window_size = size;
                }
                Task::none()
            }
        }
    }

    fn handle_key_press(&mut self, key: Key, _modifiers: Modifiers) -> Task<Message> {
        let mut changed = false;
        match key.as_ref() {
            Key::Character("j") | Key::Named(Named::ArrowDown) => {
                if self.current_page_index < (self.total_pages as usize - 1) {
                    self.current_page_index += 1;
                    changed = true;
                }
            }
            Key::Character("k") | Key::Named(Named::ArrowUp) => {
                if self.current_page_index > 0 {
                    self.current_page_index -= 1;
                    changed = true;
                }
            }
            _ => {}
        }

        if changed {
            let width = self.window_size.width as u16;
            let height = self.window_size.height as u16;
            
            let _ = self
                .render_tx
                .send(RenderCommand::RenderPage(self.current_page_index, width, height));
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if let Some(handle) = &self.current_image {
            image(handle.clone())
                .width(Length::Fill)
                .content_fit(iced::ContentFit::Contain)
                .into()
        } else {
            column![
                text("Loading...").size(20)
            ]
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub =
            keyboard::listen().map(|event| Message::EventOccurred(Event::Keyboard(event)));

        let ticker = time::every(Duration::from_millis(10)).map(|_| Message::Tick);
        
        let window_sub = window::events().map(|(id, event)| Message::WindowEvent(id, event));

        Subscription::batch(vec![keyboard_sub, ticker, window_sub])
    }
}

fn render_page_to_pixels(document: &PdfDocument, page_index: u16, target_w: u16, _target_h: u16) -> (Option<Vec<u8>>, u32, u32) {
    if let Ok(page) = document.pages().get(page_index) {
        
        let mut render_config = PdfRenderConfig::new()
            .rotate_if_landscape(PdfPageRenderRotation::None, true);
            
        if target_w > 0 {
             render_config = render_config.set_target_width(target_w.max(800) as i32);
        } else {
             render_config = render_config.set_target_width(2000);
        }

        if let Ok(bitmap) = page.render_with_config(&render_config) {
            let img = bitmap.as_image();
            let rgba = img.to_rgba8();
            let width = rgba.width();
            let height = rgba.height();
            let pixels = rgba.into_raw();
            return (Some(pixels), width, height);
        }
    }
    (None, 0, 0)
}
