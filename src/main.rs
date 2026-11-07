mod app;
mod camera;
mod elgato;
mod i18n;
mod stream;
mod usb;
mod v4l;

pub static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn main() -> eframe::Result<()> {
    START.get_or_init(std::time::Instant::now);
    let args: Vec<String> = std::env::args().collect();
    let shot = args
        .iter()
        .position(|a| a == "--screenshot")
        .and_then(|i| args.get(i + 1).cloned())
        .map(|path| app::Shot {
            path,
            frames: 0,
            requested: false,
            done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lente")
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([1000.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("Lente", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, shot)))))
}
