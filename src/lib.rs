//! Rufium - A keyboard-driven PDF reader library
//!
//! This library provides the core functionality for a Vim-like PDF reader interface.
//! It includes PDF rendering, caching, and keyboard navigation capabilities.

pub mod pdf;
pub mod ui;
pub mod input;

pub use pdf::{PdfRenderer, PdfDocument as Document};
pub use ui::{ViewerApp, ViewerConfig};
pub use input::KeyHandler;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::pdf::{PdfRenderer, PdfDocument as Document};
    pub use crate::ui::{ViewerApp, ViewerConfig};
    pub use crate::input::KeyHandler;
}
