# Rufium - Vim-like PDF Reader

A keyboard-driven PDF reader with a Vim-like interface, written in Rust.

## Features

- **Vim-like Navigation**: Intuitive keyboard controls inspired by Vim
- **Fast Rendering**: Optimized PDF rendering with intelligent page caching
- **Library and Binary**: Use as a library in your own projects or as a standalone application
- **Keyboard-First**: Minimal UI, maximum productivity

## Installation

This project uses [pdfium-render](https://github.com/ajrcarey/pdfium-render). You must provide the compiled binary library for your operating system.

1. **Download the PDFium Library**
   - Go to https://github.com/bblanchon/pdfium-binaries/releases
   - Download the package for your OS (e.g., `pdfium-linux-x64.tgz` for Linux)
   - Extract the archive
   - Copy the library into the root of this project

2. **Build the Project**
   ```bash
   cargo build --release
   ```

## Usage

### As a Binary

```bash
rufium -f /path/to/your/document.pdf
```

### As a Library

Add to your `Cargo.toml`:
```toml
[dependencies]
rufium = { path = "path/to/rufium" }
```

Example usage:
```rust
use rufium::prelude::*;

// Use the library components in your application
let app = ViewerApp::new("document.pdf".to_string());
```

## Vim-like Keybindings

### Normal Mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Next page |
| `k` / `↑` | Previous page |
| `gg` | Jump to first page |
| `G` | Jump to last page |
| `d` | Half page down (5 pages forward) |
| `u` | Half page up (5 pages backward) |
| `[number]` + `Enter` | Jump to specific page number |
| `:` | Enter command mode |
| `q` / `Q` | Quit |

### Command Mode

Enter command mode by pressing `:`, then:

| Command | Action |
|---------|--------|
| `:q` or `:quit` | Quit the application |
| `:[number]` | Jump to page [number] |
| `Esc` | Exit command mode |

## Project Structure

The project is organized as a library with a binary frontend:

- `src/lib.rs` - Library entry point
- `src/pdf.rs` - PDF rendering and document handling
- `src/input.rs` - Vim-like input handling and keybindings
- `src/ui.rs` - UI components and viewer application
- `src/main.rs` - Binary application entry point

## Performance Optimizations

- **Intelligent Caching**: Pre-renders adjacent pages for instant navigation
- **Efficient Memory Management**: Maintains a limited cache of 5 pages
- **Background Rendering**: Asynchronous page rendering doesn't block UI
- **Background Indexing**: Text indexing happens in a separate thread

## License

MIT or Apache-2.0
