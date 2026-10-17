use super::*;
use theme::{button, card, metric};

impl Viewer {
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        egui::Panel::right("inspector")
            .default_size(324.)
            .resizable(false)
            .show(ui, |ui| {
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.add_space(6.);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(language.text(Key::Inspector))
                                .small()
                                .color(ACCENT),
                        );
                        ui.label(
                            RichText::new(language.text(Key::SceneOverview))
                                .size(19.)
                                .strong(),
                        );
                    });
                });
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 6.) / 2.;
                    if button(ui, language.text(Key::ViewTab), !self.settings_tab, width).clicked()
                    {
                        self.settings_tab = false;
                    }
                    if button(ui, language.text(Key::Settings), self.settings_tab, width).clicked()
                    {
                        self.settings_tab = true;
                    }
                });
                ui.add_space(8.);
                egui::ScrollArea::vertical()
                    .id_salt("inspector_scroll")
                    .show(ui, |ui| {
                        // Keep the card's shadow inside the scroll area.
                        ui.set_width(ui.available_width() - 3.);
                        if self.settings_tab {
                            self.preferences(ui);
                        } else {
                            self.model_card(ui);
                            self.materials_card(ui);
                            self.camera_card(ui);
                            self.background_card(ui);
                            if let Some(scene) = &self.scene
                                && !scene.warnings.is_empty()
                            {
                                egui::CollapsingHeader::new(language.text(Key::ImportNotes))
                                    .id_salt("import_notes")
                                    .show(ui, |ui| {
                                        for warning in &scene.warnings {
                                            ui.label(
                                                RichText::new(warning.render(language))
                                                    .small()
                                                    .color(MUTED),
                                            );
                                        }
                                    });
                            }
                        }
                        if let Some(notice) = &self.notice {
                            ui.label(
                                RichText::new(notice.render(language))
                                    .small()
                                    .color(Color32::LIGHT_YELLOW),
                            );
                        }
                        ui.add_space(8.);
                    });
            });
    }

    fn model_card(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        card(ui, language.text(Key::Model), |ui| {
            ui.add(egui::Label::new(RichText::new(self.filename()).strong().size(14.)).wrap());
            if let Some(scene) = self.scene.clone() {
                let size = scene.bounds_for(self.plate).unwrap_or(scene.bounds).size();
                ui.label(
                    RichText::new(language.text(Key::Dimensions))
                        .small()
                        .color(MUTED),
                );
                ui.columns(3, |columns| {
                    for (column, (axis, value, color)) in columns.iter_mut().zip([
                        ("X", size.x, Color32::from_rgb(232, 138, 137)),
                        ("Y", size.y, Color32::from_rgb(150, 199, 163)),
                        ("Z", size.z, Color32::from_rgb(144, 182, 229)),
                    ]) {
                        egui::Frame::new()
                            .fill(Color32::from_rgb(26, 29, 35))
                            .corner_radius(5)
                            .inner_margin(7)
                            .show(column, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.label(RichText::new(axis).small().strong().color(color));
                                let value = if value.abs() >= 10_000. {
                                    format!("{value:.2e}")
                                } else {
                                    format!("{value:.2}")
                                };
                                ui.monospace(value);
                            });
                    }
                });
                ui.add_space(2.);
                metric(
                    ui,
                    language.text(Key::Instances),
                    format_count(scene.summaries.get(&self.plate).map_or(0, |s| s.instances)),
                );
                metric(
                    ui,
                    language.text(Key::Triangles),
                    format_count(scene.triangle_count(self.plate)),
                );
                let plates = scene
                    .summaries
                    .keys()
                    .filter_map(|p| *p)
                    .collect::<Vec<_>>();
                if !plates.is_empty() {
                    let before = self.plate;
                    ui.separator();
                    ui.label(
                        RichText::new(language.text(Key::Plates))
                            .small()
                            .color(MUTED),
                    );
                    egui::ComboBox::from_id_salt("plates")
                        .width(ui.available_width())
                        .selected_text(self.plate.map_or_else(
                            || language.text(Key::AllPlates).into(),
                            |p| Message::with(Key::Plate, [p]).render(language),
                        ))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.plate,
                                None,
                                language.text(Key::AllPlates),
                            );
                            for p in plates {
                                ui.selectable_value(
                                    &mut self.plate,
                                    Some(p),
                                    Message::with(Key::Plate, [p]).render(language),
                                );
                            }
                        });
                    if before != self.plate {
                        self.fit();
                    }
                }
            } else {
                ui.label(RichText::new(language.text(Key::NoModelHint)).color(MUTED));
            }
        });
    }

    fn materials_card(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        card(ui, language.text(Key::Materials), |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 6.) / 2.;
                for (key, project) in [(Key::ProjectColors, true), (Key::NeutralMaterial, false)] {
                    if button(
                        ui,
                        language.text(key),
                        self.settings.project_colors == project,
                        width,
                    )
                    .clicked()
                    {
                        self.settings.project_colors = project;
                        self.save_at = Some(Instant::now() + Duration::from_millis(250));
                    }
                }
            });
            if let Some(scene) = &self.scene {
                if scene.palette.is_empty() {
                    ui.label(
                        RichText::new(language.text(Key::NoProjectColors))
                            .small()
                            .color(MUTED),
                    );
                } else {
                    ui.add_space(4.);
                    egui::CollapsingHeader::new(language.text(Key::Filaments))
                        .id_salt("project_filaments")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("palette_scroll")
                                .max_height(144.)
                                .show(ui, |ui| {
                                    for (n, color) in scene.palette.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(18., 18.),
                                                egui::Sense::hover(),
                                            );
                                            let fill = color
                                                .map_or(Color32::TRANSPARENT, |[r, g, b]| {
                                                    Color32::from_rgb(r, g, b)
                                                });
                                            ui.painter().rect_filled(rect, 4., fill);
                                            ui.painter().rect_stroke(
                                                rect,
                                                4.,
                                                egui::Stroke::new(1., MUTED),
                                                egui::StrokeKind::Inside,
                                            );
                                            ui.label(
                                                RichText::new(format!("{:02}", n + 1))
                                                    .monospace()
                                                    .color(MUTED),
                                            );
                                            ui.monospace(color.map_or_else(
                                                || language.text(Key::UnknownColor).into(),
                                                settings::hex,
                                            ));
                                        });
                                    }
                                });
                        });
                }
            }
            ui.label(
                RichText::new(language.text(Key::ColorPreviewHint))
                    .small()
                    .color(MUTED),
            );
        });
    }

    fn camera_card(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        card(ui, language.text(Key::Camera), |ui| {
            ui.add_enabled_ui(self.scene.is_some(), |ui| {
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 6.) / 2.;
                    for (key, ortho) in [(Key::Perspective, false), (Key::Orthographic, true)] {
                        if button(
                            ui,
                            language.text(key),
                            self.camera.orthographic == ortho,
                            width,
                        )
                        .clicked()
                        {
                            self.camera.orthographic = ortho;
                        }
                    }
                });
                ui.add_space(2.);
                if button(
                    ui,
                    language.text(Key::Isometric),
                    self.selected_view == Some(View::Isometric),
                    ui.available_width(),
                )
                .clicked()
                {
                    self.camera.preset(View::Isometric);
                    self.selected_view = Some(View::Isometric);
                }
                let width = (ui.available_width() - 12.) / 3.;
                for views in [
                    [View::Front, View::Back, View::Top],
                    [View::Left, View::Right, View::Bottom],
                ] {
                    ui.horizontal(|ui| {
                        for view in views {
                            if button(
                                ui,
                                view.label(language),
                                self.selected_view == Some(view),
                                width,
                            )
                            .clicked()
                            {
                                self.camera.preset(view);
                                self.selected_view = Some(view);
                            }
                        }
                    });
                }
                ui.add_space(2.);
                ui.separator();
                ui.horizontal(|ui| {
                    if button(ui, "−", false, 38.)
                        .on_hover_text(language.text(Key::ZoomOut))
                        .clicked()
                    {
                        self.camera.zoom(-0.2);
                    }
                    let width = ui.available_width() - 44.;
                    if button(ui, language.text(Key::Fit), false, width)
                        .on_hover_text("F")
                        .clicked()
                    {
                        self.fit();
                    }
                    if button(ui, "+", false, 38.)
                        .on_hover_text(language.text(Key::ZoomIn))
                        .clicked()
                    {
                        self.camera.zoom(0.2);
                    }
                });
                ui.horizontal(|ui| {
                    let width = (ui.available_width() - 18.) / 4.;
                    for (key, direction) in [
                        (Key::PanLeft, -glam::Vec2::X),
                        (Key::PanUp, -glam::Vec2::Y),
                        (Key::PanDown, glam::Vec2::Y),
                        (Key::PanRight, glam::Vec2::X),
                    ] {
                        if theme::arrow(ui, direction, language.text(key), width).clicked() {
                            self.camera.keyboard_move(
                                direction,
                                self.viewport_height,
                                false,
                                false,
                            );
                        }
                    }
                });
                ui.label(
                    RichText::new(language.text(Key::PanHint))
                        .small()
                        .color(MUTED),
                );
            });
        });
    }

    fn background_card(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        card(ui, language.text(Key::Appearance), |ui| {
            ui.horizontal(|ui| {
                ui.label(language.text(Key::Background));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut color = self.settings.background;
                    if ui.color_edit_button_srgb(&mut color).changed() {
                        self.set_background(color);
                    }
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.hex)
                            .desired_width(90.)
                            .font(egui::TextStyle::Monospace)
                            .id_salt("background_hex"),
                    );
                    if response.changed()
                        && let Some(color) = settings::parse_hex(&self.hex)
                    {
                        self.settings.background = color;
                        self.save_at = Some(Instant::now() + Duration::from_millis(250));
                    }
                });
            });
            if settings::parse_hex(&self.hex).is_none() {
                ui.colored_label(Color32::LIGHT_RED, language.text(Key::ColorFormat));
            }
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 12.) / 3.;
                for (key, color) in [
                    (Key::Dark, settings::DARK),
                    (Key::Light, settings::LIGHT),
                    (Key::White, [255; 3]),
                ] {
                    if button(
                        ui,
                        language.text(key),
                        self.settings.background == color,
                        width,
                    )
                    .clicked()
                    {
                        self.set_background(color);
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .small_button(language.text(Key::DefaultBackground))
                    .clicked()
                {
                    self.set_background(settings::DARK);
                }
            });
        });
    }

    fn preferences(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        card(ui, language.text(Key::Settings), |ui| {
            ui.label(RichText::new(language.text(Key::Language)).strong());
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 6.) / 2.;
                for (label, value) in [
                    ("English", Language::English),
                    ("Русский", Language::Russian),
                ] {
                    if button(ui, label, language == value, width).clicked() {
                        self.set_language(value);
                    }
                }
            });
            ui.label(
                RichText::new(language.text(Key::LanguageHint))
                    .small()
                    .color(MUTED),
            );
        });
        self.background_card(ui);
        card(ui, language.text(Key::Help), |ui| {
            for key in [
                Key::HelpPan,
                Key::HelpKeyOrbit,
                Key::HelpMousePan,
                Key::HelpZoom,
                Key::HelpFocus,
            ] {
                ui.label(RichText::new(language.text(key)).small().color(MUTED));
            }
            if ui.button(language.text(Key::Help)).clicked() {
                self.help = true;
            }
        });
    }

    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        let language = self.settings.language;
        egui::Window::new(language.text(Key::Help))
            .id(egui::Id::unique("help_window"))
            .open(&mut self.help)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_max_width(440.);
                for key in [
                    Key::HelpOpen,
                    Key::HelpOrbit,
                    Key::HelpMousePan,
                    Key::HelpPan,
                    Key::HelpKeyOrbit,
                    Key::HelpZoom,
                    Key::HelpFit,
                    Key::HelpFocus,
                    Key::HelpEscape,
                ] {
                    ui.label(language.text(key));
                }
            });
        egui::Window::new(language.text(Key::About))
            .id(egui::Id::unique("about_window"))
            .open(&mut self.about)
            .resizable(false)
            .collapsible(false)
            .default_width(350.)
            .show(ctx, |ui| {
                ui.add_space(12.);
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(&self.branding.logo)
                            .fit_to_exact_size(Vec2::new(300., 100.))
                            .corner_radius(6)
                            .alt_text("YO3MFS"),
                    );
                    ui.label(
                        RichText::new(
                            Message::with(Key::AboutVersion, [env!("CARGO_PKG_VERSION")])
                                .render(language),
                        )
                        .small()
                        .color(MUTED),
                    );
                    ui.add_space(12.);
                    ui.label(language.text(Key::AboutDescription));
                    ui.add_space(8.);
                    ui.hyperlink_to(language.text(Key::MadeInAustin), "https://aomega.co/");
                    ui.add_space(12.);
                    ui.label(
                        RichText::new(language.text(Key::AboutIndependent))
                            .small()
                            .color(MUTED),
                    );
                    ui.add_space(6.);
                });
            });
    }
}
