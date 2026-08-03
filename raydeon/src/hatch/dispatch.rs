//! Per-surface-kind hatch dispatch: turning one [`HatchStyle`] into the
//! passes a `Planar`, `Sphere`, or `Revolution` surface draws them as, then
//! eroding each pass's lines by the scene's lighting.
//!
//! `hatch::hatch_shape` is the only caller — it already knows which surface
//! kind it holds and calls straight into the matching function here; this
//! module owns what each kind DOES with a style, not which kind a shape has.

use super::jitter::{jitter01, world_seed};
use super::lines;
use super::style::{HatchCoverage, HatchSpacing, HatchStyle};
use super::surface::{PlanarSurface, RevolutionSurface, SphereSurface};
use crate::scene::SceneCamera;
use crate::{DrawableShape, LineSegment3D, WPoint3, WVec3, WorldSpace};
use euclid::Angle;

pub(super) fn hatch_planar(
    cam: &SceneCamera,
    shape: &DrawableShape,
    surface: &PlanarSurface,
    style: &HatchStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    let normal = surface.normal();
    let passes: Vec<(Angle<f64>, HatchSpacing, Keep)> = match style {
        HatchStyle::Tonal { passes } => passes
            .iter()
            .map(|pass| {
                (
                    pass.angle,
                    pass.spacing,
                    Keep::Below(pass.threshold.into_inner()),
                )
            })
            .collect(),
        HatchStyle::Stochastic {
            angle,
            spacing,
            coverage,
        } => vec![(*angle, *spacing, Keep::jittered(*coverage, cam.seed()))],
        HatchStyle::LightFlow {
            source,
            spacing,
            cross_spacing,
            cross_threshold,
            coverage,
        } => {
            let flow = surface.in_plane_angle(*source - surface.centroid());
            vec![
                (flow, *spacing, Keep::jittered(*coverage, cam.seed())),
                (
                    flow + Angle::degrees(90.0),
                    *cross_spacing,
                    Keep::Below(cross_threshold.into_inner()),
                ),
            ]
        }
    };

    passes
        .into_iter()
        .flat_map(|(angle, spacing, keep)| {
            lines::planar(surface, angle, spacing)
                .iter()
                .flat_map(|line| shade(cam, shape, line, |_| normal, keep))
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn hatch_sphere(
    cam: &SceneCamera,
    shape: &DrawableShape,
    surface: &SphereSurface,
    style: &HatchStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    let center = surface.center();
    let passes: Vec<(WVec3, HatchSpacing, Keep)> = match style {
        // A sphere has no single direction to hatch along, so tonal passes
        // become contour rings about the three axes, darkest last. A fourth
        // pass would only overdraw rings already on the sphere.
        HatchStyle::Tonal { passes } => passes
            .iter()
            .zip(ring_axes())
            .map(|(pass, axis)| (axis, pass.spacing, Keep::Below(pass.threshold.into_inner())))
            .collect(),
        HatchStyle::Stochastic {
            spacing, coverage, ..
        } => vec![(
            WVec3::new(0.0, 0.0, 1.0),
            *spacing,
            Keep::jittered(*coverage, cam.seed()),
        )],
        HatchStyle::LightFlow {
            source,
            spacing,
            coverage,
            ..
        } => vec![(
            center - *source,
            *spacing,
            Keep::jittered(*coverage, cam.seed()),
        )],
    };

    passes
        .into_iter()
        .flat_map(|(axis, spacing, keep)| {
            lines::sphere_rings(surface, axis, spacing)
                .iter()
                .flat_map(|ring| {
                    shade(cam, shape, ring, |point| (point - center).normalize(), keep)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Which kind of curve a revolution hatch pass draws.
#[derive(Debug, Copy, Clone)]
enum RevolutionLine {
    /// A latitude ring, stepped around the surface by profile arc length.
    Ring,
    /// A profile curve, stepped around the axis by angle.
    Meridian,
}

pub(super) fn hatch_revolution(
    cam: &SceneCamera,
    shape: &DrawableShape,
    surface: &RevolutionSurface,
    style: &HatchStyle,
) -> Vec<LineSegment3D<WorldSpace>> {
    let passes: Vec<(RevolutionLine, HatchSpacing, Keep)> = match style {
        // The natural cross-hatch on a thrown form: rings and meridians
        // alternate pass to pass, darkest last.
        HatchStyle::Tonal { passes } => passes
            .iter()
            .enumerate()
            .map(|(ndx, pass)| {
                let line = if ndx % 2 == 0 {
                    RevolutionLine::Ring
                } else {
                    RevolutionLine::Meridian
                };
                (line, pass.spacing, Keep::Below(pass.threshold.into_inner()))
            })
            .collect(),
        HatchStyle::Stochastic {
            spacing, coverage, ..
        } => vec![(
            RevolutionLine::Ring,
            *spacing,
            Keep::jittered(*coverage, cam.seed()),
        )],
        // A projected flow direction has no stable meaning on a closed
        // revolution, so one jittered rings pass stands in for the flow
        // pass, and the cross-below-threshold pass is dropped — exactly as
        // the sphere path above does.
        HatchStyle::LightFlow {
            spacing, coverage, ..
        } => vec![(
            RevolutionLine::Ring,
            *spacing,
            Keep::jittered(*coverage, cam.seed()),
        )],
    };

    passes
        .into_iter()
        .flat_map(|(line, spacing, keep)| {
            let curves = match line {
                RevolutionLine::Ring => lines::revolution_rings(surface, spacing),
                RevolutionLine::Meridian => lines::revolution_meridians(surface, spacing),
            };
            curves
                .iter()
                .flat_map(|curve| {
                    shade(
                        cam,
                        shape,
                        curve,
                        |point| surface.normal_at_point(point),
                        keep,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The axes tonal passes wrap a sphere's rings around, in the order the
/// passes darken. Passes beyond the third have no axis left to wrap.
fn ring_axes() -> [WVec3; 3] {
    [
        WVec3::new(0.0, 0.0, 1.0),
        WVec3::new(1.0, 0.0, 0.0),
        WVec3::new(0.0, 1.0, 0.0),
    ]
}

/// When a pass keeps a sample of a hatch line.
#[derive(Debug, Copy, Clone)]
enum Keep {
    /// Draw wherever the surface is darker than this tone.
    Below(f64),
    /// Draw with a probability which rises as the tone falls, scattered by
    /// the sample's own position so the result is reproducible.
    Jittered { coverage: f64, seed: u64 },
}

impl Keep {
    fn jittered(coverage: HatchCoverage, seed: u64) -> Self {
        Keep::Jittered {
            coverage: coverage.into_inner(),
            seed,
        }
    }

    fn keeps(&self, tone: f64, point: WPoint3) -> bool {
        match self {
            Keep::Below(threshold) => tone < *threshold,
            Keep::Jittered { coverage, seed } => {
                tone < jitter01(world_seed(point, *seed)) * coverage
            }
        }
    }
}

fn shade(
    cam: &SceneCamera,
    shape: &DrawableShape,
    line: &LineSegment3D<WorldSpace>,
    normal_at: impl Fn(WPoint3) -> WVec3,
    keep: Keep,
) -> Vec<LineSegment3D<WorldSpace>> {
    lines::filter_by_tone(
        cam.scene(),
        shape,
        cam.eye(),
        line,
        normal_at,
        |tone, at| keep.keeps(tone, at),
    )
}
