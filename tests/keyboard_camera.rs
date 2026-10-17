use glam::{Vec2, Vec3};
use yo3mfs::camera::{Camera, View};

#[test]
fn keyboard_pan_moves_on_screen_without_rotating_in_every_view() {
    for view in View::ALL {
        for ortho in [false, true] {
            for height in [400., 800.] {
                for direction in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                    let mut c = Camera::default();
                    c.preset(view);
                    c.orthographic = ortho;
                    let rotation = c.rotation;
                    let before = c.matrix(1.).project_point3(Vec3::ZERO);
                    c.keyboard_move(direction, height, false, false);
                    let after = c.matrix(1.).project_point3(Vec3::ZERO);
                    let pixels = Vec2::new(after.x - before.x, before.y - after.y) * height / 2.;
                    assert!(
                        (pixels - direction * 24.).length() < 0.001,
                        "{view:?}: {pixels:?}"
                    );
                    assert_eq!(rotation, c.rotation);
                }
            }
        }
    }
}

#[test]
fn shift_speeds_up_pan_and_alt_orbits_without_changing_target() {
    let mut normal = Camera::default();
    let mut fast = normal.clone();
    normal.keyboard_move(Vec2::X, 600., false, false);
    fast.keyboard_move(Vec2::X, 600., false, true);
    assert!((fast.target - normal.target * 4.).length() < 1e-6);
    let mut orbit = Camera::default();
    let rotation = orbit.rotation;
    orbit.keyboard_move(Vec2::X, 600., true, false);
    assert_eq!(orbit.target, Vec3::ZERO);
    assert_ne!(rotation, orbit.rotation);
}
