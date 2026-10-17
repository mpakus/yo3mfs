use crate::{
    camera::{Camera, View},
    i18n::{Key, Language, Message},
    loader::Loader,
    renderer::{GpuScene, Renderer},
    scene::Scene,
    settings::{self, Settings},
};
use eframe::{
    egui::{self, Color32, Rect, RichText, Vec2},
    egui_wgpu,
};
use std::{
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

mod menus;
mod panels;
mod theme;

const ACCENT: Color32 = Color32::from_rgb(232, 169, 139);
const MUTED: Color32 = Color32::from_rgb(163, 171, 184);

pub struct Viewer {
    branding: crate::branding::Branding,
    state: egui_wgpu::RenderState,
    renderer: Arc<Renderer>,
    gpu: Option<Arc<GpuScene>>,
    scene: Option<Arc<Scene>>,
    loader: Loader,
    camera: Camera,
    path: Option<PathBuf>,
    loading_path: Option<PathBuf>,
    error: Option<UiError>,
    settings: Settings,
    settings_path: Option<PathBuf>,
    hex: String,
    save_at: Option<Instant>,
    notice: Option<Message>,
    plate: Option<u32>,
    aspect: f32,
    load_started: Option<Instant>,
    load_seconds: Option<f64>,
    help: bool,
    about: bool,
    settings_tab: bool,
    selected_view: Option<View>,
    viewport_height: f32,
    gpu_errors: mpsc::Receiver<Message>,
}
enum UiError {
    Import(crate::import::ImportError),
    Message(Message),
}
impl UiError {
    fn render(&self, language: Language) -> String {
        match self {
            Self::Import(e) => e.localized(language),
            Self::Message(m) => m.render(language),
        }
    }
}
impl Viewer {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let state = cc
            .wgpu_render_state
            .clone()
            .ok_or(Message::from(Key::GpuInit))?;
        let renderer = Renderer::new(&state)?;
        let (gpu_tx, gpu_errors) = mpsc::sync_channel(1);
        let lost_tx = gpu_tx.clone();
        let wake = cc.egui_ctx.clone();
        state
            .device
            .set_device_lost_callback(move |reason, message| {
                if reason != eframe::wgpu::DeviceLostReason::Destroyed {
                    let _ = lost_tx.try_send(Message::with(Key::GpuLost, [message]));
                    wake.request_repaint();
                }
            });
        let wake = cc.egui_ctx.clone();
        state.device.on_uncaptured_error(Arc::new(move |error| {
            let _ = gpu_tx.try_send(Message::with(Key::GpuError, [error]));
            wake.request_repaint();
        }));
        theme::apply(&cc.egui_ctx);
        let settings_path = settings::path();
        let (settings, notice) = settings_path
            .as_deref()
            .map(settings::read)
            .unwrap_or_else(|| (Settings::default(), Some(Key::SettingsMissing.into())));
        let wake = cc.egui_ctx.clone();
        let loader = Loader::new(move || wake.request_repaint())?;
        let mut viewer = Self {
            branding: crate::branding::Branding::load(&cc.egui_ctx)?,
            state,
            renderer,
            gpu: None,
            scene: None,
            loader,
            camera: Camera::default(),
            path: None,
            loading_path: None,
            error: None,
            hex: settings::hex(settings.background),
            settings,
            settings_path,
            save_at: None,
            notice,
            plate: None,
            aspect: 1.5,
            load_started: None,
            load_seconds: None,
            help: false,
            about: false,
            settings_tab: false,
            selected_view: Some(View::Isometric),
            viewport_height: 600.,
            gpu_errors,
        };
        if let Some(path) = initial {
            viewer.open(path);
        }
        Ok(viewer)
    }
    fn open(&mut self, path: PathBuf) {
        let resident = self.scene.as_ref().map_or(0, |s| s.estimated_bytes())
            + self.gpu.as_ref().map_or(0, |s| s.bytes);
        self.loader.open(path.clone(), resident);
        self.loading_path = Some(path);
        self.error = None;
        self.load_started = Some(Instant::now());
    }
    fn dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(self.t(Key::Open))
            .add_filter(self.t(Key::FileFilter), &["3mf", "3MF"])
            .pick_file()
        {
            self.open(path);
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(loaded) = self.loader.poll() {
            self.loading_path = None;
            match loaded.result {
                Ok(prepared) => match Renderer::upload(&self.state.device, &prepared) {
                    Ok(gpu) => {
                        self.gpu = Some(gpu);
                        self.scene = Some(Arc::new(prepared.scene));
                        self.path = Some(loaded.path);
                        self.plate = None;
                        self.camera = Camera::default();
                        self.selected_view = Some(View::Isometric);
                        self.fit();
                        self.load_seconds = self.load_started.map(|t| t.elapsed().as_secs_f64());
                        self.error = None;
                        let title = format!("{} — YO3MFS", self.filename());
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                    }
                    Err(e) => self.error = Some(UiError::Message(e)),
                },
                Err(e) => self.error = Some(UiError::Import(e)),
            }
        }
        if self.save_at.is_some_and(|at| Instant::now() >= at) {
            self.persist();
        }
        if self.loader.loading || self.save_at.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    fn filename(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.t(Key::NoModel).into())
    }
    fn fit(&mut self) {
        if let Some(scene) = &self.scene
            && let Some(b) = scene.bounds_for(self.plate)
        {
            let radius = scene.bounds.radius();
            self.camera.fit(
                ((b.center() - scene.bounds.center()) / radius).as_vec3(),
                (b.radius() / radius) as f32,
                self.aspect,
            );
        }
    }
    fn reset(&mut self) {
        self.camera = Camera::default();
        self.selected_view = Some(View::Isometric);
        self.fit();
    }
    fn set_background(&mut self, color: [u8; 3]) {
        self.settings.background = color;
        self.hex = settings::hex(color);
        self.save_at = Some(Instant::now() + Duration::from_millis(250));
    }
    fn persist(&mut self) {
        self.save_at = None;
        if let Some(path) = &self.settings_path
            && let Err(e) = settings::save(path, &self.settings)
        {
            self.notice = Some(Message::with(Key::SaveFailed, [e]));
        }
    }
    fn t(&self, key: Key) -> &'static str {
        self.settings.language.text(key)
    }
    fn set_language(&mut self, language: Language) {
        self.settings.language = language;
        self.persist();
    }
    fn canvas(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                self.viewport_height = rect.height();
                self.aspect = (rect.width() / rect.height().max(1.)).max(0.01);
                let background = Color32::from_rgb(
                    self.settings.background[0],
                    self.settings.background[1],
                    self.settings.background[2],
                );
                ui.painter().rect_filled(rect, 0., background);
                if let Some(gpu) = self.gpu.clone() {
                    if response.clicked() {
                        response.request_focus();
                    }
                    if !egui::Popup::is_any_open(ui.ctx()) && !self.help && !self.about {
                        let delta = ui.input(|i| i.pointer.delta());
                        if response.dragged_by(egui::PointerButton::Middle)
                            || (response.dragged_by(egui::PointerButton::Primary)
                                && ui.input(|i| i.modifiers.shift))
                        {
                            self.camera
                                .pan(glam::Vec2::new(delta.x, delta.y), rect.height());
                        } else if response.dragged_by(egui::PointerButton::Primary) {
                            self.camera.orbit(glam::Vec2::new(delta.x, delta.y));
                            self.selected_view = None;
                        }
                        if response.hovered() {
                            let (scroll, pinch) =
                                ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                            if (pinch - 1.).abs() > 0.0001 {
                                self.camera.zoom(pinch.ln());
                            } else if scroll != 0. {
                                self.camera.zoom(scroll * 0.0015);
                            }
                        }
                        if response.has_focus() {
                            self.camera_keys(ui, &response);
                        }
                    }
                    ui.painter().add(self.renderer.callback(
                        rect,
                        gpu.clone(),
                        self.camera.matrix(self.aspect),
                        self.plate,
                    ));
                    self.axes(ui, rect, background);
                } else {
                    let light = self
                        .settings
                        .background
                        .iter()
                        .map(|&v| v as u32)
                        .sum::<u32>()
                        > 450;
                    let fg = if light {
                        Color32::from_rgb(39, 49, 61)
                    } else {
                        Color32::from_rgb(224, 230, 238)
                    };
                    ui.painter().image(
                        self.branding.icon.id(),
                        Rect::from_center_size(
                            rect.center() + Vec2::new(0., -105.),
                            Vec2::splat(144.),
                        ),
                        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        rect.center() + Vec2::new(0., -12.),
                        egui::Align2::CENTER_CENTER,
                        self.t(Key::EmptyTitle),
                        egui::FontId::proportional(27.),
                        fg,
                    );
                    ui.painter().text(
                        rect.center() + Vec2::new(0., 28.),
                        egui::Align2::CENTER_CENTER,
                        self.t(Key::EmptyHint),
                        egui::FontId::proportional(15.),
                        fg.gamma_multiply(0.7),
                    );
                    let button = Rect::from_center_size(
                        rect.center() + Vec2::new(0., 85.),
                        Vec2::new(180., 42.),
                    );
                    if ui
                        .put(
                            button,
                            egui::Button::new(RichText::new(self.t(Key::Open)).strong())
                                .fill(theme::ACTIVE),
                        )
                        .clicked()
                    {
                        self.dialog();
                    }
                }
                if !ui.input(|i| i.raw.hovered_files.is_empty()) {
                    ui.painter()
                        .rect_filled(rect, 0., Color32::from_black_alpha(175));
                    ui.painter().rect_stroke(
                        rect.shrink(18.),
                        8.,
                        egui::Stroke::new(2., ACCENT),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        self.t(Key::DropHint),
                        egui::FontId::proportional(24.),
                        Color32::WHITE,
                    );
                }
            });
    }
    fn camera_keys(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.fit();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Home)) {
            self.reset();
        }
        if pan_keys(ui, response, &mut self.camera, self.viewport_height) {
            self.selected_view = None;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            self.camera.zoom(0.2);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Minus)) {
            self.camera.zoom(-0.2);
        }
    }
    fn axes(&self, ui: &egui::Ui, rect: Rect, background: Color32) {
        let center = rect.left_bottom() + Vec2::new(54., -55.);
        let rotation = self.camera.rotation.inverse();
        for (label, axis, color) in [
            ("X", glam::Vec3::X, Color32::from_rgb(235, 106, 109)),
            ("Y", glam::Vec3::Y, Color32::from_rgb(117, 210, 148)),
            ("Z", glam::Vec3::Z, Color32::from_rgb(110, 170, 250)),
        ] {
            let direction = rotation * axis;
            let end = center + Vec2::new(direction.x, -direction.y) * 29.;
            ui.painter()
                .line_segment([center, end], egui::Stroke::new(2., color));
            ui.painter().circle_filled(end, 9., background);
            ui.painter().text(
                end,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(12.),
                color,
            );
        }
    }
}
impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Ok(error) = self.gpu_errors.try_recv() {
            self.loader.cancel();
            rfd::MessageDialog::new()
                .set_title(self.t(Key::GpuTitle))
                .set_description(
                    Message::with(Key::GpuRestart, [error.render(self.settings.language)])
                        .render(self.settings.language),
                )
                .set_level(rfd::MessageLevel::Error)
                .show();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.poll(&ctx);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.dialog();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma)) {
            self.settings_tab = true;
            egui::Popup::close_all(&ctx);
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)
                || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)
        }) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::F1)) {
            self.help = true;
            egui::Popup::close_all(&ctx);
        }
        if !ctx.egui_wants_keyboard_input()
            && !egui::Popup::is_any_open(&ctx)
            && !self.help
            && !self.about
        {
            if ctx.input(|i| i.key_pressed(egui::Key::F)) {
                self.fit();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                self.reset();
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !egui::Popup::is_any_open(&ctx) {
            if self.help || self.about {
                self.help = false;
                self.about = false;
            } else {
                self.loader.cancel();
                self.loading_path = None;
            }
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            if dropped.len() == 1 {
                self.open(dropped[0].path().to_path_buf());
            } else {
                self.error = Some(UiError::Message(Key::OneFile.into()));
            }
        }
        self.menu_bar(ui);
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.loader.loading {
                    ui.spinner();
                    ui.label(self.t(self.loader.stage()));
                    if ui.small_button(self.t(Key::Cancel)).clicked() {
                        self.loader.cancel();
                        self.loading_path = None;
                    }
                } else {
                    ui.label(
                        RichText::new(self.t(Key::StatusControls))
                            .small()
                            .color(MUTED),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(seconds) = self.load_seconds {
                        ui.label(
                            RichText::new(
                                Message::with(Key::Seconds, [format!("{seconds:.2}")])
                                    .render(self.settings.language),
                            )
                            .small()
                            .color(MUTED),
                        );
                    }
                    ui.label(
                        RichText::new(self.t(Key::LocalReadOnly))
                            .small()
                            .color(ACCENT),
                    );
                });
            });
        });
        if let Some(error) = self
            .error
            .as_ref()
            .map(|e| e.render(self.settings.language))
        {
            egui::Panel::top("error").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(Color32::LIGHT_RED, &error);
                    if ui.small_button(self.t(Key::Close)).clicked() {
                        self.error = None;
                    }
                    if ui.small_button(self.t(Key::OtherFile)).clicked() {
                        self.dialog();
                    }
                });
            });
        }
        self.sidebar(ui);
        self.canvas(ui);
        self.dialogs(&ctx);
    }

    fn on_exit(&mut self) {
        if self.save_at.is_some() {
            self.persist();
        }
        self.loader.cancel();
    }
}
/// Returns whether the arrow gesture changed orientation.
fn pan_keys(ui: &egui::Ui, response: &egui::Response, camera: &mut Camera, height: f32) -> bool {
    if !response.has_focus() || egui::Popup::is_any_open(ui.ctx()) {
        return false;
    }
    lock_camera_focus(ui, response.id);
    let delta = ui.input(|i| {
        let mut d = glam::Vec2::ZERO;
        for (key, v) in [
            (egui::Key::ArrowLeft, glam::Vec2::new(-1., 0.)),
            (egui::Key::ArrowRight, glam::Vec2::new(1., 0.)),
            (egui::Key::ArrowUp, glam::Vec2::new(0., -1.)),
            (egui::Key::ArrowDown, glam::Vec2::new(0., 1.)),
        ] {
            if i.key_pressed(key) {
                d += v;
            }
        }
        d
    });
    let modifiers = ui.input(|i| i.modifiers);
    if !modifiers.command && !modifiers.ctrl && delta != glam::Vec2::ZERO {
        camera.keyboard_move(delta, height, modifiers.alt, modifiers.shift);
        return modifiers.alt;
    }
    false
}
fn lock_camera_focus(ui: &egui::Ui, id: egui::Id) {
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                horizontal_arrows: true,
                vertical_arrows: true,
                ..Default::default()
            },
        )
    });
}
fn format_count(n: usize) -> String {
    let s = n.to_string();
    s.chars()
        .enumerate()
        .fold(String::new(), |mut out, (i, c)| {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push(' ');
            }
            out.push(c);
            out
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arrow_events_pan_viewport_and_leave_text_input_alone() {
        let ctx = egui::Context::default();
        let mut camera = Camera::default();
        let mut hex = "#20242B".to_string();
        let rotation = camera.rotation;
        let mut frame = |focus_view: bool, focus_text: bool, press: bool| {
            let mut raw = egui::RawInput::default();
            if press {
                raw.events.push(egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                });
            }
            let output = ctx.run_ui(raw, |ui| {
                let text = ui.add(egui::TextEdit::singleline(&mut hex));
                let (_, response) =
                    ui.allocate_exact_size(Vec2::new(400., 300.), egui::Sense::click_and_drag());
                if focus_view {
                    response.request_focus();
                }
                if focus_text {
                    text.request_focus();
                }
                pan_keys(ui, &response, &mut camera, 300.);
            });
            output.drop_without_applying_deltas();
            camera.target
        };
        frame(true, false, false);
        let moved = frame(false, false, true);
        assert!(moved.length() > 0.);
        frame(false, true, false);
        assert_eq!(moved, frame(false, false, true));
        assert_eq!(camera.rotation, rotation);
        assert_eq!(hex, "#20242B");
    }
}
