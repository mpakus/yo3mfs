#![forbid(unsafe_code)]
use yo3mfs::i18n::{Key, Message};

fn main() {
    let language = yo3mfs::settings::path()
        .as_deref()
        .map(yo3mfs::settings::read)
        .map(|(s, _)| s.language)
        .unwrap_or_default();
    let mut args = std::env::args_os().skip(1);
    let initial = args.next();
    if initial.as_deref() == Some(std::ffi::OsStr::new("--help")) {
        println!(
            "yo3mfs {}\nUsage: yo3mfs [path/to/model.3mf]\nOffline 3MF viewer. Input files are never modified.",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    if initial.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("yo3mfs {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.next().is_some() {
        eprintln!("{}", language.text(Key::OneFile));
        std::process::exit(2);
    }
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1280., 800.])
        .with_min_inner_size([800., 600.])
        .with_app_id(yo3mfs::branding::APP_ID)
        .with_drag_and_drop(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(yo3mfs::branding::ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        depth_buffer: 32,
        multisampling: 4,
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "YO3MFS",
        options,
        Box::new(move |cc| {
            Ok(Box::new(yo3mfs::app::Viewer::new(
                cc,
                initial.map(std::path::PathBuf::from),
            )?))
        }),
    ) {
        eprintln!("{}: {error}", language.text(Key::LaunchTitle));
        rfd::MessageDialog::new()
            .set_title(language.text(Key::LaunchTitle))
            .set_description(Message::with(Key::LaunchError, [error]).render(language))
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}
