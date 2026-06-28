mod app;
mod converter;
mod metadata;
mod models;
mod parser;
mod ui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 620.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("CBZ Converter")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "cbz-converter",
        native_options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::AppState::new(cc)))
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Korean font embedded in the binary so Hangul renders on any OS/distro
    // regardless of which system fonts are installed.
    fonts.font_data.insert(
        "korean".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/NanumGothic.ttf")).into(),
    );

    // Append as fallback: default fonts handle Latin first, the embedded font
    // covers Hangul glyphs they lack.
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .unwrap()
            .push("korean".to_owned());
    }

    ctx.set_fonts(fonts);
}
