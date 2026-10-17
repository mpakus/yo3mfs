//! Independent decoder for Bambu's hexadecimal facet-paint wire format.
//! The format and geometric conventions are recorded in docs/BAMBU-COLORS.md.
use super::{ImportError, Limits, check_cancel, invalid};
use crate::i18n::Key;
use glam::DVec3;
use std::sync::atomic::AtomicBool;

pub(crate) fn visit(
    code: &str,
    triangle: [DVec3; 3],
    limits: &Limits,
    cancel: &AtomicBool,
    mut emit: impl FnMut([DVec3; 3], u16),
) -> Result<usize, ImportError> {
    let mut digits = code.bytes().rev();
    let mut nibble = || match digits.next() {
        Some(c @ b'0'..=b'9') => Ok(c - b'0'),
        Some(c @ b'A'..=b'F') => Ok(c - b'A' + 10),
        _ => Err(invalid(Key::BadPaint)),
    };
    check_cancel(cancel)?;
    let mut pending = vec![(triangle, 0usize)];
    let mut leaves = 0usize;
    let mut nodes = 0usize;
    while let Some((points, depth)) = pending.pop() {
        nodes += 1;
        if nodes.is_multiple_of(1024) {
            check_cancel(cancel)?;
        }
        let token = nibble()?;
        let divisions = token & 3;
        let upper = token >> 2;
        if divisions == 0 {
            let mut filament = u16::from(upper);
            if upper == 3 {
                loop {
                    let next = nibble()?;
                    filament += u16::from(next);
                    if filament > 256 {
                        return Err(invalid(Key::BadPaint));
                    }
                    if next != 15 {
                        break;
                    }
                }
            }
            leaves += 1;
            if leaves > limits.rendered_triangles {
                return Err(ImportError::Limit(Key::RenderedTriangles));
            }
            emit(points, filament);
            continue;
        }
        if upper > 2 || (divisions == 3 && upper != 0) {
            return Err(invalid(Key::BadPaint));
        }
        if depth >= limits.paint_depth.min(64) {
            return Err(ImportError::Limit(Key::PaintDepth));
        }
        let side = usize::from(upper);
        let [a, b, c] = std::array::from_fn(|n| points[(n + side) % 3]);
        let ab = (a + b) * 0.5;
        let bc = (b + c) * 0.5;
        let ca = (c + a) * 0.5;
        let children = match divisions {
            1 => [[a, b, bc], [bc, c, a], [a; 3], [a; 3]],
            2 => [[a, ab, ca], [ab, b, ca], [b, c, ca], [a; 3]],
            _ => [[a, ab, ca], [ab, b, bc], [bc, c, ca], [ab, bc, ca]],
        };
        // A LIFO stack visits the last child first, as required by the file format.
        for child in children.into_iter().take(usize::from(divisions) + 1) {
            pending.push((child, depth + 1));
        }
    }
    if digits.next().is_some() {
        return Err(invalid(Key::BadPaint));
    }
    Ok(leaves)
}

#[cfg(test)]
mod tests {
    use super::*;
    const TRIANGLE: [DVec3; 3] = [DVec3::ZERO, DVec3::new(4., 0., 0.), DVec3::new(0., 4., 0.)];

    #[test]
    fn plain_and_extended_filament_codes() {
        for (code, expected) in [
            ("0", 0),
            ("4", 1),
            ("8", 2),
            ("0C", 3),
            ("1C", 4),
            ("0FC", 18),
            ("0FFC", 33),
        ] {
            let mut actual = Vec::new();
            visit(
                code,
                TRIANGLE,
                &Limits::default(),
                &AtomicBool::new(false),
                |t, f| actual.push((t, f)),
            )
            .unwrap();
            assert_eq!(actual, vec![(TRIANGLE, expected)]);
        }
    }

    #[test]
    fn splits_preserve_area_winding_and_child_color_positions() {
        for code in ["481", "485", "489", "480C2", "480C6", "480CA", "480C03"] {
            let mut leaves = Vec::new();
            visit(
                code,
                TRIANGLE,
                &Limits::default(),
                &AtomicBool::new(false),
                |t, f| leaves.push((t, f)),
            )
            .unwrap();
            let area: f64 = leaves
                .iter()
                .map(|(t, _)| (t[1] - t[0]).cross(t[2] - t[0]).z * 0.5)
                .sum();
            assert_eq!(area, 8., "{code}");
            assert!(
                leaves
                    .iter()
                    .all(|(t, _)| (t[1] - t[0]).cross(t[2] - t[0]).z > 0.)
            );
            if code == "481" {
                assert_eq!(
                    leaves,
                    vec![
                        ([DVec3::new(2., 2., 0.), TRIANGLE[2], TRIANGLE[0]], 2),
                        ([TRIANGLE[0], TRIANGLE[1], DVec3::new(2., 2., 0.)], 1)
                    ]
                );
            }
            if code == "480C03" {
                assert_eq!(
                    leaves.iter().map(|(_, f)| *f).collect::<Vec<_>>(),
                    [0, 3, 2, 1]
                );
                assert_eq!(
                    leaves[0].0,
                    [
                        DVec3::new(2., 0., 0.),
                        DVec3::new(2., 2., 0.),
                        DVec3::new(0., 2., 0.)
                    ]
                );
            }
        }
    }

    #[test]
    fn malformed_truncated_and_excessive_paint_are_bounded() {
        for code in [
            "", "c", "C", "FC", "1", "44", "F", "D", "7", "garbage", "é", "FFFFFC",
        ] {
            assert!(
                visit(
                    code,
                    TRIANGLE,
                    &Limits::default(),
                    &AtomicBool::new(false),
                    |_, _| {}
                )
                .is_err(),
                "{code}"
            );
        }
        let limits = Limits {
            paint_depth: 1,
            ..Limits::default()
        };
        assert!(matches!(
            visit("11", TRIANGLE, &limits, &AtomicBool::new(false), |_, _| {}),
            Err(ImportError::Limit(Key::PaintDepth))
        ));
        let limits = Limits {
            rendered_triangles: 1,
            ..Limits::default()
        };
        assert!(matches!(
            visit("481", TRIANGLE, &limits, &AtomicBool::new(false), |_, _| {}),
            Err(ImportError::Limit(Key::RenderedTriangles))
        ));
        assert!(matches!(
            visit(
                "4",
                TRIANGLE,
                &Limits::default(),
                &AtomicBool::new(true),
                |_, _| {}
            ),
            Err(ImportError::Cancelled)
        ));
    }
}
