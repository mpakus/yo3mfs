use super::*;

impl Viewer {
    pub(super) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let language = self.settings.language;
        egui::Panel::top("menu_bar")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 24.;
                ui.spacing_mut().item_spacing.x = 16.;
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button(RichText::new("YO3MFS").strong(), |ui| {
                        ui.set_min_width(220.);
                        if item(ui, language.text(Key::AboutApp), None) {
                            self.about = true;
                        }
                        ui.separator();
                        if item(
                            ui,
                            language.text(Key::Preferences),
                            command(egui::Key::Comma),
                        ) {
                            self.settings_tab = true;
                        }
                        ui.separator();
                        if item(ui, language.text(Key::Quit), command(egui::Key::Q)) {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(language.text(Key::File), |ui| {
                        ui.set_min_width(220.);
                        if item(ui, language.text(Key::OpenMenu), command(egui::Key::O)) {
                            self.dialog();
                        }
                        ui.separator();
                        if item(ui, language.text(Key::CloseWindow), command(egui::Key::W)) {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(language.text(Key::ViewTab), |ui| {
                        ui.set_min_width(220.);
                        ui.add_enabled_ui(self.scene.is_some(), |ui| {
                            if item(ui, language.text(Key::Fit), key(egui::Key::F)) {
                                self.fit();
                            }
                            if item(ui, language.text(Key::ResetView), key(egui::Key::Home)) {
                                self.reset();
                            }
                            ui.separator();
                            if item(ui, language.text(Key::ZoomIn), key(egui::Key::Plus)) {
                                self.camera.zoom(0.2);
                            }
                            if item(ui, language.text(Key::ZoomOut), key(egui::Key::Minus)) {
                                self.camera.zoom(-0.2);
                            }
                            ui.separator();
                            ui.menu_button(language.text(Key::Orientation), |ui| {
                                for view in View::ALL {
                                    if ui
                                        .selectable_label(
                                            self.selected_view == Some(view),
                                            view.label(language),
                                        )
                                        .clicked()
                                    {
                                        self.camera.preset(view);
                                        self.selected_view = Some(view);
                                        egui::Popup::close_all(ui.ctx());
                                    }
                                }
                            });
                            ui.menu_button(language.text(Key::Projection), |ui| {
                                for (label, ortho) in
                                    [(Key::Perspective, false), (Key::Orthographic, true)]
                                {
                                    if ui
                                        .selectable_label(
                                            self.camera.orthographic == ortho,
                                            language.text(label),
                                        )
                                        .clicked()
                                    {
                                        self.camera.orthographic = ortho;
                                        egui::Popup::close_all(ui.ctx());
                                    }
                                }
                            });
                        });
                    });
                    ui.menu_button(language.text(Key::HelpMenu), |ui| {
                        ui.set_min_width(220.);
                        if item(ui, language.text(Key::Help), key(egui::Key::F1)) {
                            self.help = true;
                        }
                    });
                });
            });
    }
}

fn command(key: egui::Key) -> Option<egui::KeyboardShortcut> {
    Some(egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key))
}
fn key(key: egui::Key) -> Option<egui::KeyboardShortcut> {
    Some(egui::KeyboardShortcut::new(egui::Modifiers::NONE, key))
}
fn item(ui: &mut egui::Ui, label: &str, shortcut: Option<egui::KeyboardShortcut>) -> bool {
    let mut button = egui::Button::new(label);
    if let Some(shortcut) = shortcut {
        let mac = ui.ctx().os().is_mac();
        // The bundled symbol font includes ⌘. Keep other modifiers textual
        // until their glyphs are included too, and show punctuation as keys.
        let mut names = if mac && shortcut.modifiers == egui::Modifiers::COMMAND {
            egui::ModifierNames::SYMBOLS
        } else {
            egui::ModifierNames::NAMES
        };
        names.is_short = true;
        button = button.shortcut_text(shortcut.format(&names, mac));
    }
    let clicked = ui.add(button).clicked();
    if clicked {
        ui.close();
    }
    clicked
}
