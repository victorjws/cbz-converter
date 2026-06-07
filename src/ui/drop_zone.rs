pub fn render(ui: &mut egui::Ui) {
    let hovered = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    let available = ui.available_rect_before_wrap();

    if hovered {
        ui.painter().rect_filled(
            available,
            8.0,
            egui::Color32::from_rgba_unmultiplied(100, 150, 255, 40),
        );
        ui.painter().rect_stroke(
            available,
            8.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
            egui::StrokeKind::Outside,
        );
    }

    ui.centered_and_justified(|ui| {
        let (text, color) = if hovered {
            ("Drop here", egui::Color32::from_rgb(100, 150, 255))
        } else {
            ("Drag folders here", egui::Color32::GRAY)
        };
        ui.add(egui::Label::new(
            egui::RichText::new(text).size(22.0).color(color),
        ));
    });
}
