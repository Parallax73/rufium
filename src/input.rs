//! Input handling and Vim-like keybindings

use iced::keyboard::key::Named;
use iced::keyboard::Key;

/// Vim-like navigation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMode {
    /// Normal mode - for navigation
    Normal,
    /// Command mode - for entering commands with `:` prefix
    Command,
}

/// Navigation action that results from key input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationAction {
    /// Move to next page
    NextPage,
    /// Move to previous page
    PrevPage,
    /// Jump to first page
    FirstPage,
    /// Jump to last page
    LastPage,
    /// Scroll down half page (Ctrl+d in Vim)
    HalfPageDown,
    /// Scroll up half page (Ctrl+u in Vim)
    HalfPageUp,
    /// Jump to specific page
    JumpToPage(usize),
    /// Enter command mode
    EnterCommandMode,
    /// Exit/quit
    Quit,
    /// No action
    None,
}

/// Key handler for Vim-like navigation
pub struct KeyHandler {
    mode: NavigationMode,
    command_buffer: String,
}

impl KeyHandler {
    pub fn new() -> Self {
        Self {
            mode: NavigationMode::Normal,
            command_buffer: String::new(),
        }
    }

    pub fn mode(&self) -> NavigationMode {
        self.mode
    }

    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    /// Process a key press and return the corresponding action
    pub fn handle_key(&mut self, key: &Key) -> NavigationAction {
        match self.mode {
            NavigationMode::Normal => self.handle_normal_mode(key),
            NavigationMode::Command => self.handle_command_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: &Key) -> NavigationAction {
        match key.as_ref() {
            // Next page: j, Down arrow, Ctrl+f
            Key::Character("j") | Key::Named(Named::ArrowDown) => NavigationAction::NextPage,
            
            // Previous page: k, Up arrow, Ctrl+b
            Key::Character("k") | Key::Named(Named::ArrowUp) => NavigationAction::PrevPage,
            
            // First page: gg (handled via number buffer)
            Key::Character("g") => {
                if self.command_buffer == "g" {
                    self.command_buffer.clear();
                    NavigationAction::FirstPage
                } else {
                    self.command_buffer = "g".to_string();
                    NavigationAction::None
                }
            }
            
            // Last page: G (Shift+g)
            Key::Character("G") => NavigationAction::LastPage,
            
            // Half page down: Ctrl+d
            Key::Character("d") => NavigationAction::HalfPageDown,
            
            // Half page up: Ctrl+u  
            Key::Character("u") => NavigationAction::HalfPageUp,
            
            // Quit: q, ZZ, Ctrl+c
            Key::Character("q") | Key::Character("Q") => NavigationAction::Quit,
            
            // Enter command mode: :
            Key::Character(":") => {
                self.mode = NavigationMode::Command;
                self.command_buffer.clear();
                NavigationAction::EnterCommandMode
            }
            
            // Number input for page jump
            Key::Character(c) if c.chars().all(|ch| ch.is_numeric()) => {
                self.command_buffer.push_str(c);
                NavigationAction::None
            }
            
            // Enter to execute number jump
            Key::Named(Named::Enter) if !self.command_buffer.is_empty() => {
                if let Ok(page_num) = self.command_buffer.parse::<usize>() {
                    self.command_buffer.clear();
                    NavigationAction::JumpToPage(page_num)
                } else {
                    self.command_buffer.clear();
                    NavigationAction::None
                }
            }
            
            // Escape to clear buffer
            Key::Named(Named::Escape) => {
                self.command_buffer.clear();
                NavigationAction::None
            }
            
            _ => NavigationAction::None,
        }
    }

    fn handle_command_mode(&mut self, key: &Key) -> NavigationAction {
        match key.as_ref() {
            // Execute command
            Key::Named(Named::Enter) => {
                let action = self.parse_command();
                self.mode = NavigationMode::Normal;
                self.command_buffer.clear();
                action
            }
            
            // Cancel command mode
            Key::Named(Named::Escape) => {
                self.mode = NavigationMode::Normal;
                self.command_buffer.clear();
                NavigationAction::None
            }
            
            // Backspace
            Key::Named(Named::Backspace) => {
                self.command_buffer.pop();
                if self.command_buffer.is_empty() {
                    self.mode = NavigationMode::Normal;
                }
                NavigationAction::None
            }
            
            // Character input
            Key::Character(c) => {
                self.command_buffer.push_str(c);
                NavigationAction::None
            }
            
            _ => NavigationAction::None,
        }
    }

    fn parse_command(&self) -> NavigationAction {
        let cmd = self.command_buffer.trim();
        
        // :q or :quit - quit
        if cmd == "q" || cmd == "quit" {
            return NavigationAction::Quit;
        }
        
        // :123 - jump to page 123
        if let Ok(page_num) = cmd.parse::<usize>() {
            return NavigationAction::JumpToPage(page_num);
        }
        
        NavigationAction::None
    }
}

impl Default for KeyHandler {
    fn default() -> Self {
        Self::new()
    }
}
