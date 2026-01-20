//! UI components and viewer application

use crate::input::{KeyHandler, NavigationAction, NavigationMode};
use crate::pdf::{init_pdfium, PdfRenderer};
use iced::keyboard::{self, Modifiers};
use iced::widget::{column, container, image, text};
use iced::{time, window, Element, Event, Length, Size, Subscription, Task};
use std::collections::HashMap;
use std::process;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Configuration for the PDF viewer
pub struct ViewerConfig {
    pub initial_window_width: f32,
    pub initial_window_height: f32,
    pub cache_size: usize,
    pub half_page_scroll_amount: usize,
    pub target_render_height: f32,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            initial_window_width: 800.0,
            initial_window_height: 600.0,
            cache_size: 5,
            half_page_scroll_amount: 5,
            target_render_height: 800.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    EventOccurred(Event),
    WindowEvent(window::Id, window::Event),
}

enum RenderCommand {
    RenderPage(usize, u16, u16),
}

struct RenderResult {
    page_index: usize,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

/// Main viewer application
pub struct ViewerApp {
    _file_name: String,
    current_image: Option<image::Handle>,
    current_page_index: usize,
    total_pages: u16,
    render_tx: mpsc::Sender<RenderCommand>,
    render_rx: Arc<Mutex<mpsc::Receiver<RenderResult>>>,
    _search_index: Arc<Mutex<Vec<String>>>,
    window_size: Size,
    window_id: Option<window::Id>,
    page_cache: HashMap<usize, image::Handle>,
    key_handler: KeyHandler,
    config: ViewerConfig,
}

impl ViewerApp {
    /// Create a new viewer application for the given PDF file
    pub fn new(file_name: String) -> (Self, Task<Message>) {
        Self::with_config(file_name, ViewerConfig::default())
    }

    /// Create a new viewer with custom configuration
    pub fn with_config(file_name: String, config: ViewerConfig) -> (Self, Task<Message>) {
        let file_name_for_render = file_name.clone();
        let file_name_for_index = file_name.clone();

        let total_pages = {
            let pdfium = match init_pdfium() {
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
        let (ui_tx, ui_rx) = mpsc::channel::<RenderResult>();

        // Indexing thread - builds search index in background
        let index_store = search_index.clone();
        thread::spawn(move || {
            let pdfium = match init_pdfium() {
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
            }
        });

        // Rendering thread - handles page rendering requests
        thread::spawn(move || {
            let pdfium = match init_pdfium() {
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
                        if let Some((pixels, width, height)) =
                            PdfRenderer::render_page_to_pixels(&document, idx as u16, w, h)
                        {
                            let _ = ui_tx.send(RenderResult {
                                page_index: idx,
                                pixels,
                                width,
                                height,
                            });
                        }
                    }
                }
            }
        });

        // Initial render
        render_tx
            .send(RenderCommand::RenderPage(
                0,
                config.initial_window_width as u16,
                config.initial_window_height as u16,
            ))
            .unwrap();

        (
            Self {
                _file_name: file_name,
                current_image: None,
                current_page_index: 0,
                total_pages,
                render_tx,
                render_rx: Arc::new(Mutex::new(ui_rx)),
                _search_index: search_index,
                window_size: Size::new(
                    config.initial_window_width,
                    config.initial_window_height,
                ),
                window_id: None,
                page_cache: HashMap::new(),
                key_handler: KeyHandler::new(),
                config,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => self.handle_tick(),
            Message::EventOccurred(event) => {
                if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
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

    fn handle_tick(&mut self) -> Task<Message> {
        if let Ok(rx) = self.render_rx.lock() {
            while let Ok(result) = rx.try_recv() {
                let handle = image::Handle::from_rgba(result.width, result.height, result.pixels);

                self.page_cache.insert(result.page_index, handle.clone());
                
                // Optimize cache: keep only nearby pages
                if self.page_cache.len() > self.config.cache_size {
                    let keys: Vec<usize> = self.page_cache.keys().copied().collect();
                    let mut to_remove = Vec::new();
                    for key in keys {
                        if key < self.current_page_index.saturating_sub(2)
                            || key > self.current_page_index + 2
                        {
                            to_remove.push(key);
                            if self.page_cache.len() - to_remove.len() <= self.config.cache_size {
                                break;
                            }
                        }
                    }
                    for key in to_remove {
                        self.page_cache.remove(&key);
                    }
                }

                if result.page_index == self.current_page_index {
                    self.current_image = Some(handle.clone());

                    let aspect_ratio = result.width as f32 / result.height as f32;
                    let new_height = self.config.target_render_height;
                    let new_width = new_height * aspect_ratio;

                    if let Some(id) = self.window_id {
                        return window::resize(id, Size::new(new_width, new_height));
                    }
                }
            }
        }
        Task::none()
    }

    fn handle_key_press(
        &mut self,
        key: iced::keyboard::Key,
        _modifiers: Modifiers,
    ) -> Task<Message> {
        let action = self.key_handler.handle_key(&key);

        match action {
            NavigationAction::NextPage => {
                if self.current_page_index < (self.total_pages as usize - 1) {
                    self.current_page_index += 1;
                    self.render_current_and_adjacent_pages();
                }
            }
            NavigationAction::PrevPage => {
                if self.current_page_index > 0 {
                    self.current_page_index -= 1;
                    self.render_current_and_adjacent_pages();
                }
            }
            NavigationAction::FirstPage => {
                self.current_page_index = 0;
                self.render_current_and_adjacent_pages();
            }
            NavigationAction::LastPage => {
                self.current_page_index = (self.total_pages as usize).saturating_sub(1);
                self.render_current_and_adjacent_pages();
            }
            NavigationAction::HalfPageDown => {
                // Move forward by configured amount (half page scroll simulation)
                let new_index = (self.current_page_index + self.config.half_page_scroll_amount)
                    .min(self.total_pages as usize - 1);
                self.current_page_index = new_index;
                self.render_current_and_adjacent_pages();
            }
            NavigationAction::HalfPageUp => {
                // Move backward by configured amount (half page scroll simulation)
                let new_index = self
                    .current_page_index
                    .saturating_sub(self.config.half_page_scroll_amount);
                self.current_page_index = new_index;
                self.render_current_and_adjacent_pages();
            }
            NavigationAction::JumpToPage(page_num) => {
                let target = page_num.saturating_sub(1);
                if target < self.total_pages as usize {
                    self.current_page_index = target;
                    self.render_current_and_adjacent_pages();
                }
            }
            NavigationAction::Quit => {
                process::exit(0);
            }
            NavigationAction::EnterCommandMode | NavigationAction::None => {}
        }

        Task::none()
    }

    fn render_current_and_adjacent_pages(&mut self) {
        let width = self.window_size.width as u16;
        let height = self.window_size.height as u16;

        // Check cache first for current page
        if let Some(cached) = self.page_cache.get(&self.current_page_index) {
            self.current_image = Some(cached.clone());
        } else {
            let _ = self.render_tx.send(RenderCommand::RenderPage(
                self.current_page_index,
                width,
                height,
            ));
        }

        // Pre-render previous page
        if self.current_page_index > 0
            && !self.page_cache.contains_key(&(self.current_page_index - 1))
        {
            let _ = self.render_tx.send(RenderCommand::RenderPage(
                self.current_page_index - 1,
                width,
                height,
            ));
        }

        // Pre-render next page
        if self.current_page_index < (self.total_pages as usize - 1)
            && !self.page_cache.contains_key(&(self.current_page_index + 1))
        {
            let _ = self.render_tx.send(RenderCommand::RenderPage(
                self.current_page_index + 1,
                width,
                height,
            ));
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let image_area: Element<'_, Message> = if let Some(handle) = &self.current_image {
            container(
                image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Color::BLACK.into()),
                ..container::Style::default()
            })
            .into()
        } else {
            container(text("Loading...").size(20).color(iced::Color::WHITE))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_theme| container::Style {
                    background: Some(iced::Color::BLACK.into()),
                    ..container::Style::default()
                })
                .into()
        };

        // Enhanced status bar with mode indicator
        let status_text = match self.key_handler.mode() {
            NavigationMode::Command => {
                format!(":{}", self.key_handler.command_buffer())
            }
            NavigationMode::Normal => {
                let buffer = self.key_handler.command_buffer();
                if buffer.is_empty() {
                    format!("Page {} / {}", self.current_page_index + 1, self.total_pages)
                } else {
                    format!(
                        "Page {} / {} | {}",
                        self.current_page_index + 1,
                        self.total_pages,
                        buffer
                    )
                }
            }
        };

        let mode_indicator = match self.key_handler.mode() {
            NavigationMode::Normal => "-- NORMAL --",
            NavigationMode::Command => "-- COMMAND --",
        };

        let status_bar = container(
            column![
                text(mode_indicator).size(12).color(iced::Color::from_rgb8(100, 200, 100)),
                text(status_text).size(14).color(iced::Color::WHITE),
            ]
        )
        .width(Length::Fill)
        .padding(5)
        .style(|_theme| container::Style {
            background: Some(iced::Color::from_rgb8(30, 30, 30).into()),
            ..container::Style::default()
        });

        column![image_area, status_bar].into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub =
            keyboard::listen().map(|event| Message::EventOccurred(Event::Keyboard(event)));

        let ticker = time::every(Duration::from_millis(50)).map(|_| Message::Tick);

        let window_sub = window::events().map(|(id, event)| Message::WindowEvent(id, event));

        Subscription::batch(vec![keyboard_sub, ticker, window_sub])
    }
}
