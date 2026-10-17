use crate::i18n::{Key, Language};
use glam::{Mat4, Quat, Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Isometric,
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
}
impl View {
    pub const ALL: [Self; 7] = [
        Self::Isometric,
        Self::Front,
        Self::Back,
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
    ];
    pub fn label(self, language: Language) -> &'static str {
        language.text(match self {
            Self::Isometric => Key::Isometric,
            Self::Front => Key::Front,
            Self::Back => Key::Back,
            Self::Left => Key::Left,
            Self::Right => Key::Right,
            Self::Top => Key::Top,
            Self::Bottom => Key::Bottom,
        })
    }
}
/// Camera coordinates are normalized around the scene bounding sphere.
#[derive(Clone, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub rotation: Quat,
    pub distance: f32,
    pub orthographic: bool,
    pub radius: f32,
}
impl Default for Camera {
    fn default() -> Self {
        let mut c = Self {
            target: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            distance: 4.,
            orthographic: false,
            radius: 1.,
        };
        c.preset(View::Isometric);
        c
    }
}
impl Camera {
    pub fn preset(&mut self, view: View) {
        let (eye, up) = match view {
            View::Isometric => (Vec3::new(1.4, -1.7, 1.2), Vec3::Z),
            View::Front => (-Vec3::Y, Vec3::Z),
            View::Back => (Vec3::Y, Vec3::Z),
            View::Left => (-Vec3::X, Vec3::Z),
            View::Right => (Vec3::X, Vec3::Z),
            View::Top => (Vec3::Z, Vec3::Y),
            View::Bottom => (-Vec3::Z, Vec3::Y),
        };
        self.rotation =
            Quat::from_mat4(&glam::camera::rh::view::look_at_mat4(eye, Vec3::ZERO, up).inverse())
                .normalize();
    }
    pub fn fit(&mut self, center: Vec3, radius: f32, aspect: f32) {
        self.target = center;
        self.radius = radius.max(1e-6);
        let vertical = 22.5_f32.to_radians();
        let half = vertical.min((vertical.tan() * aspect.max(0.01)).atan());
        self.distance = self.radius / half.sin() * 1.12;
    }
    pub fn orbit(&mut self, delta: Vec2) {
        self.rotation = (Quat::from_rotation_z(-delta.x * 0.006)
            * self.rotation
            * Quat::from_rotation_x(-delta.y * 0.006))
        .normalize();
    }
    pub fn zoom(&mut self, amount: f32) {
        if amount.is_finite() {
            self.distance = (self.distance * (-amount.clamp(-2., 2.)).exp())
                .clamp(self.radius * 0.02, self.radius * 10_000.);
        }
    }
    pub fn pan(&mut self, delta: Vec2, height: f32) {
        let units = 2. * self.distance * 22.5_f32.to_radians().tan() / height.max(1.);
        self.target += self.rotation * Vec3::new(-delta.x, delta.y, 0.) * units;
    }
    /// Directions use screen coordinates: right +X, down +Y.
    pub fn keyboard_move(&mut self, direction: Vec2, height: f32, orbit: bool, fast: bool) {
        let speed = if fast { 4. } else { 1. };
        if orbit {
            self.orbit(direction * 10. * speed);
        } else {
            self.pan(direction * 24. * speed, height);
        }
    }
    pub fn matrix(&self, aspect: f32) -> Mat4 {
        let aspect = aspect.max(0.01);
        let eye = self.target + self.rotation * Vec3::Z * self.distance;
        let view = glam::camera::rh::view::look_at_mat4(eye, self.target, self.rotation * Vec3::Y);
        let near = (self.distance * 0.0001).max(self.radius * 0.00001);
        let far = self.distance + self.target.length() + self.radius * 100.;
        let projection = if self.orthographic {
            let h = self.distance * 22.5_f32.to_radians().tan();
            glam::camera::rh::proj::directx::orthographic(-h * aspect, h * aspect, -h, h, near, far)
        } else {
            glam::camera::rh::proj::directx::perspective(45_f32.to_radians(), aspect, near, far)
        };
        projection * view
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_sides_and_projections_fit_the_sphere() {
        for view in View::ALL {
            for ortho in [false, true] {
                for aspect in [0.5, 1., 2.] {
                    let mut c = Camera::default();
                    c.preset(view);
                    c.orthographic = ortho;
                    c.fit(Vec3::ZERO, 3_f32.sqrt(), aspect);
                    let m = c.matrix(aspect);
                    for x in [-1., 1.] {
                        for y in [-1., 1.] {
                            for z in [-1., 1.] {
                                let p = m.project_point3(Vec3::new(x, y, z));
                                assert!(
                                    p.x.abs() <= 1. && p.y.abs() <= 1. && p.z >= 0. && p.z <= 1.,
                                    "{view:?} {p:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn gestures_remain_finite_at_extremes() {
        let mut c = Camera::default();
        for _ in 0..1000 {
            c.orbit(Vec2::new(4., 3.));
            c.zoom(2.);
        }
        c.pan(Vec2::ONE, 0.);
        assert!(c.matrix(0.).is_finite());
        assert!(c.distance > 0.);
        let before = c.distance;
        c.zoom(f32::NAN);
        assert_eq!(before, c.distance);
    }
}
