use super::{ACCENT, Color32, MUTED, RichText, Vec2, egui};

pub(super) const ACTIVE: Color32 = Color32::from_rgb(83, 59, 49);
const BORDER: Color32 = Color32::from_rgb(66, 72, 81);

pub(super) fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "symbols".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("symbols".into());
    ctx.set_fonts(fonts);
    ctx.set_visuals(egui::Visuals::dark());
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(6., 8.);
    style.spacing.button_padding = Vec2::new(10., 7.);
    style.spacing.interact_size.y = 30.;
    style.visuals.panel_fill = Color32::from_rgb(27, 30, 36);
    style.visuals.window_fill = Color32::from_rgb(34, 38, 45);
    style.visuals.extreme_bg_color = Color32::from_rgb(23, 26, 31);
    style.visuals.faint_bg_color = Color32::from_rgb(40, 44, 52);
    style.visuals.selection.bg_fill = ACTIVE;
    style.visuals.selection.stroke = egui::Stroke::new(1., ACCENT);
    style.visuals.hyperlink_color = ACCENT;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(47, 52, 61);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1., BORDER);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(62, 65, 73);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1., ACCENT.gamma_multiply(0.65));
    style.visuals.widgets.active.weak_bg_fill = ACTIVE;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1., ACCENT);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.corner_radius = 5.into();
    }
    for kind in [egui::TextStyle::Body, egui::TextStyle::Button] {
        style
            .text_styles
            .insert(kind, egui::FontId::proportional(13.));
    }
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(11.));
    ctx.set_global_style(style);
}

pub(super) fn arrow(
    ui: &mut egui::Ui,
    direction: glam::Vec2,
    label: &str,
    width: f32,
) -> egui::Response {
    let response = ui.add_sized([width, 30.], egui::Button::new(""));
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
    let vector = Vec2::new(direction.x, direction.y) * 12.;
    ui.painter().arrow(
        response.rect.center() - vector * 0.5,
        vector,
        egui::Stroke::new(1.5, ui.visuals().text_color()),
    );
    response.on_hover_text(label)
}

pub(super) fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_rgb(36, 40, 47))
        .stroke(egui::Stroke::new(1., BORDER))
        .corner_radius(9)
        .inner_margin(12)
        .shadow(egui::epaint::Shadow {
            offset: [0, 2],
            blur: 5,
            spread: 0,
            color: Color32::from_black_alpha(35),
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(10.).strong().color(MUTED));
            ui.add_space(2.);
            body(ui);
        });
    ui.add_space(4.);
}

pub(super) fn button(ui: &mut egui::Ui, label: &str, selected: bool, width: f32) -> egui::Response {
    ui.add_sized([width, 30.], egui::Button::new(label).selected(selected))
}

pub(super) fn metric(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.monospace(value);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn about_symbol_and_both_languages_have_glyphs() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let output = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|fonts| {
                assert!(fonts.has_glyphs(
                    &egui::FontId::proportional(13.),
                    "Made in Austin ✩ Texas · English · Русский ⌘"
                ));
            });
        });
        output.drop_without_applying_deltas();
    }
}
