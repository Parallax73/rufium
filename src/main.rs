use clap::Parser;
use rufium::ui::ViewerApp;
use std::path::Path;
use std::process;

#[derive(Parser, Debug, Clone)]
#[command(version)]
struct Args {
    #[arg(short = 'f', long)]
    file_name: String,
}

pub fn main() -> iced::Result {
    let args = Args::parse();
    let path = Path::new(&args.file_name);

    if path.extension().and_then(|e| e.to_str()) != Some("pdf") {
        eprintln!("The file you entered isn't a PDF.");
        process::exit(1);
    }

    iced::application(
        move || ViewerApp::new(args.file_name.clone()),
        ViewerApp::update,
        ViewerApp::view,
    )
    .title("Rufium - Vim-like PDF Reader")
    .subscription(ViewerApp::subscription)
    .run()
}
