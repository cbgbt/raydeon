//! Surfaces of revolution: a radial profile spun around an axis.
//!
//! This is the geometry behind a wheel-thrown form (vase, bowl, column): a
//! foot-to-lip profile of `(radius, height)` points, spun a full turn around
//! an axis through a base point.

use crate::hatch::lines::ring_frame;
use crate::{WPoint3, WVec3};
use euclid::Angle;
use snafu::prelude::*;

/// Below this length an axis vector is indistinguishable from nothing.
const MIN_AXIS_LENGTH: f64 = 1.0e-9;

/// A point of a lathe profile: radius out from the axis at a height along it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ProfilePoint {
    pub radius: f64,
    pub height: f64,
}

/// Why a description of a revolution profile describes no surface.
#[derive(Debug, Snafu)]
pub enum RevolutionSurfaceError {
    #[snafu(display(
        "the axis must be non-zero to describe a direction to spin the profile around"
    ))]
    DegenerateAxis,
    #[snafu(display("a profile needs at least two points, foot to lip, but had {points}"))]
    ProfileTooShort { points: usize },
    #[snafu(display("profile point {index} has a non-finite radius or height"))]
    NonFiniteProfile { index: usize },
    #[snafu(display(
        "profile point {index} has radius {radius}, but a radius must be finite and positive"
    ))]
    NonPositiveRadius { index: usize, radius: f64 },
    #[snafu(display(
        "profile point {index} does not rise above the one before it; profile heights must \
         strictly increase from foot to lip"
    ))]
    NonMonotonicHeight { index: usize },
}

/// A surface of revolution: a radial profile spun around an axis through
/// `base`.
///
/// The fields are private because well-formedness of the axis and profile —
/// unit length, strictly increasing height — is what lets [`RevolutionSurface::normal_at`]
/// prove its outward direction: a profile segment's normal in the (radial,
/// axial) frame is `(dh, -dr)` normalized, the foot→lip tangent turned a
/// quarter turn so a rising wall's normal points away from the axis. Strict
/// height monotonicity gives `dh > 0` on every segment, so every segment
/// normal — and every vertex average of adjacent ones, a convex combination —
/// has a positive radial component: outward is provably away from the axis,
/// not merely a convention an overhanging profile could silently flip.
/// [`RevolutionSurface::try_new`] is the only way to mint one.
#[derive(Debug, Clone)]
pub struct RevolutionSurface {
    base: WPoint3,
    axis: WVec3,
    profile: Vec<ProfilePoint>,
}

impl RevolutionSurface {
    /// Parses a surface of revolution from a base point, a spin axis, and a
    /// profile of points from foot to lip.
    ///
    /// The axis is normalized. The profile must have at least two points,
    /// each with a finite positive radius and finite height, and heights
    /// must strictly increase from foot to lip.
    pub fn try_new(
        base: WPoint3,
        axis: WVec3,
        profile: Vec<ProfilePoint>,
    ) -> Result<Self, RevolutionSurfaceError> {
        let axis_length = axis.length();
        ensure!(axis_length > MIN_AXIS_LENGTH, DegenerateAxisSnafu);
        let axis = axis / axis_length;

        ensure!(
            profile.len() >= 2,
            ProfileTooShortSnafu {
                points: profile.len()
            }
        );

        for (index, point) in profile.iter().enumerate() {
            ensure!(
                point.radius.is_finite() && point.height.is_finite(),
                NonFiniteProfileSnafu { index }
            );
            ensure!(
                point.radius > 0.0,
                NonPositiveRadiusSnafu {
                    index,
                    radius: point.radius,
                }
            );
        }
        for index in 1..profile.len() {
            ensure!(
                profile[index].height > profile[index - 1].height,
                NonMonotonicHeightSnafu { index }
            );
        }

        Ok(Self {
            base,
            axis,
            profile,
        })
    }

    pub fn base(&self) -> WPoint3 {
        self.base
    }

    pub fn axis(&self) -> WVec3 {
        self.axis
    }

    pub fn profile(&self) -> &[ProfilePoint] {
        &self.profile
    }

    /// Total arc length of the profile, foot to lip.
    pub(crate) fn arc_length(&self) -> f64 {
        self.profile
            .windows(2)
            .map(|pair| segment_length(pair[0], pair[1]))
            .sum()
    }

    /// The largest radius the profile reaches: the surface never spins wider
    /// than this.
    pub(crate) fn max_radius(&self) -> f64 {
        self.profile
            .iter()
            .fold(0.0_f64, |max, point| max.max(point.radius))
    }

    /// The world position at arc length `t` along the profile (clamped to
    /// `[0, arc_length()]`), spun to `angle` around the axis.
    pub(crate) fn point_at(&self, t: f64, angle: Angle<f64>) -> WPoint3 {
        let located = self.locate(t);
        self.base + self.axis * located.height + self.radial_dir(angle) * located.radius
    }

    /// The outward normal at arc length `t` along the profile, spun to
    /// `angle`. See the type's own docs for why this is provably away from
    /// the axis.
    pub(crate) fn normal_at(&self, t: f64, angle: Angle<f64>) -> WVec3 {
        let located = self.locate(t);
        self.radial_dir(angle) * located.normal2d.0 + self.axis * located.normal2d.1
    }

    /// The outward normal at a world point at or near the surface, found by
    /// recovering its height along the axis and its angle about the axis.
    /// Height and arc length vary linearly together within a single profile
    /// segment, so locating by height finds the same point `normal_at`
    /// would from the corresponding `t`.
    pub(crate) fn normal_at_point(&self, point: WPoint3) -> WVec3 {
        let offset = point - self.base;
        let height = offset.dot(self.axis);
        let radial = offset - self.axis * height;
        let (u, v) = ring_frame(self.axis);
        let angle = Angle::radians(radial.dot(v).atan2(radial.dot(u)));

        let located = self.locate_by_height(height);
        self.radial_dir(angle) * located.normal2d.0 + self.axis * located.normal2d.1
    }

    /// A unit vector at `angle` in the plane perpendicular to the axis.
    fn radial_dir(&self, angle: Angle<f64>) -> WVec3 {
        let (u, v) = ring_frame(self.axis);
        u * angle.radians.cos() + v * angle.radians.sin()
    }

    /// The 2D normal (average of adjacent segment normals) at profile vertex
    /// `index`.
    fn vertex_normal2d(&self, index: usize) -> (f64, f64) {
        let last_segment = self.profile.len() - 2;
        let prev =
            (index > 0).then(|| segment_normal2d(self.profile[index - 1], self.profile[index]));
        let next = (index <= last_segment)
            .then(|| segment_normal2d(self.profile[index], self.profile[index + 1]));

        match (prev, next) {
            (Some(p), Some(n)) => normalize2d((p.0 + n.0, p.1 + n.1)),
            (Some(p), None) => p,
            (None, Some(n)) => n,
            (None, None) => unreachable!("try_new requires at least two profile points"),
        }
    }

    /// Radius, height and 2D normal at arc length `t`, clamped to the
    /// profile's own arc length range.
    fn locate(&self, t: f64) -> Located {
        let t = t.clamp(0.0, self.arc_length());
        let mut cursor = 0.0;
        for index in 0..self.profile.len() - 1 {
            let seg_len = segment_length(self.profile[index], self.profile[index + 1]);
            if t <= cursor + seg_len || index == self.profile.len() - 2 {
                let fraction = if seg_len > 0.0 {
                    ((t - cursor) / seg_len).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return self.located_in_segment(index, fraction);
            }
            cursor += seg_len;
        }
        unreachable!("try_new requires at least two profile points")
    }

    /// Radius, height and 2D normal at a given height, clamped to the
    /// profile's own height range.
    fn locate_by_height(&self, height: f64) -> Located {
        let first = self.profile[0].height;
        let last = self.profile[self.profile.len() - 1].height;
        let height = height.clamp(first, last);

        for index in 0..self.profile.len() - 1 {
            let (h0, h1) = (self.profile[index].height, self.profile[index + 1].height);
            if height <= h1 || index == self.profile.len() - 2 {
                let fraction = ((height - h0) / (h1 - h0)).clamp(0.0, 1.0);
                return self.located_in_segment(index, fraction);
            }
        }
        unreachable!("try_new requires at least two profile points")
    }

    fn located_in_segment(&self, index: usize, fraction: f64) -> Located {
        let a = self.profile[index];
        let b = self.profile[index + 1];
        let radius = lerp(a.radius, b.radius, fraction);
        let height = lerp(a.height, b.height, fraction);
        let normal_a = self.vertex_normal2d(index);
        let normal_b = self.vertex_normal2d(index + 1);
        let normal2d = normalize2d((
            lerp(normal_a.0, normal_b.0, fraction),
            lerp(normal_a.1, normal_b.1, fraction),
        ));
        Located {
            radius,
            height,
            normal2d,
        }
    }
}

/// A resolved point along the profile: radius, height, and the outward 2D
/// normal in the (radial, axial) frame.
struct Located {
    radius: f64,
    height: f64,
    normal2d: (f64, f64),
}

fn segment_length(a: ProfilePoint, b: ProfilePoint) -> f64 {
    let dr = b.radius - a.radius;
    let dh = b.height - a.height;
    (dr * dr + dh * dh).sqrt()
}

/// The 2D outward normal of the segment from `a` to `b`: the foot→lip
/// tangent `(dr, dh)` turned a quarter turn to `(dh, -dr)`, normalized.
fn segment_normal2d(a: ProfilePoint, b: ProfilePoint) -> (f64, f64) {
    let dr = b.radius - a.radius;
    let dh = b.height - a.height;
    normalize2d((dh, -dr))
}

fn normalize2d(v: (f64, f64)) -> (f64, f64) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    (v.0 / len, v.1 / len)
}

fn lerp(a: f64, b: f64, fraction: f64) -> f64 {
    a + (b - a) * fraction
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn point(radius: f64, height: f64) -> ProfilePoint {
        ProfilePoint { radius, height }
    }

    fn straight_wall() -> Vec<ProfilePoint> {
        vec![point(1.0, 0.0), point(1.0, 1.0)]
    }

    #[test]
    fn a_well_formed_profile_is_a_surface() {
        assert!(RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            straight_wall(),
        )
        .is_ok());
    }

    #[test]
    fn the_axis_is_normalized() {
        let surface =
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 5.0), straight_wall())
                .expect("a straight wall is a surface");
        assert_eq!(surface.axis(), WVec3::new(0.0, 0.0, 1.0));
    }

    #[test_case(WVec3::zero(); "zero axis")]
    #[test_case(WVec3::new(1.0e-12, 0.0, 0.0); "near-zero axis")]
    fn a_degenerate_axis_is_no_surface(axis: WVec3) {
        assert!(matches!(
            RevolutionSurface::try_new(WPoint3::zero(), axis, straight_wall()),
            Err(RevolutionSurfaceError::DegenerateAxis)
        ));
    }

    #[test_case(vec![]; "empty")]
    #[test_case(vec![point(1.0, 0.0)]; "single point")]
    fn a_short_profile_is_no_surface(profile: Vec<ProfilePoint>) {
        assert!(matches!(
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile),
            Err(RevolutionSurfaceError::ProfileTooShort { .. })
        ));
    }

    #[test_case(f64::NAN, 0.0; "nan radius")]
    #[test_case(f64::INFINITY, 0.0; "infinite radius")]
    #[test_case(1.0, f64::NAN; "nan height")]
    fn a_non_finite_profile_point_is_no_surface(radius: f64, height: f64) {
        let profile = vec![point(1.0, 0.0), point(radius, height)];
        assert!(matches!(
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile),
            Err(RevolutionSurfaceError::NonFiniteProfile { index: 1 })
        ));
    }

    #[test_case(0.0; "zero radius")]
    #[test_case(-1.0; "negative radius")]
    fn a_non_positive_radius_is_no_surface(radius: f64) {
        let profile = vec![point(1.0, 0.0), point(radius, 1.0)];
        assert!(matches!(
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile),
            Err(RevolutionSurfaceError::NonPositiveRadius { index: 1, .. })
        ));
    }

    #[test_case(vec![point(1.0, 1.0), point(1.0, 1.0)]; "duplicate height")]
    #[test_case(vec![point(1.0, 1.0), point(1.0, 0.0)]; "decreasing height")]
    fn a_non_monotonic_profile_is_no_surface(profile: Vec<ProfilePoint>) {
        assert!(matches!(
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile),
            Err(RevolutionSurfaceError::NonMonotonicHeight { index: 1 })
        ));
    }

    #[test]
    fn a_vertical_wall_normal_points_radially_away_from_the_axis() {
        let surface =
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), straight_wall())
                .expect("a straight wall is a surface");

        for angle_deg in [0.0, 45.0, 90.0, 180.0, 270.0] {
            let angle = Angle::degrees(angle_deg);
            let normal = surface.normal_at(0.5, angle);
            let radial = surface.radial_dir(angle);
            assert!(
                normal.dot(radial) > 0.0,
                "normal at angle {angle_deg} should point away from the axis, got {normal:?}"
            );
            assert!(
                (normal.length() - 1.0).abs() < 1.0e-9,
                "normal should be unit length, got {normal:?}"
            );
        }
    }

    #[test]
    fn a_flared_wall_normal_still_points_away_from_the_axis() {
        // A rising, outward-flaring profile: dr > 0 and dh > 0 on the one
        // segment, still strictly monotone in height.
        let surface = RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            vec![point(0.5, 0.0), point(1.0, 1.0)],
        )
        .expect("a flared wall is a surface");

        for t in [0.0, 0.3, 0.7, 1.0] {
            let angle = Angle::degrees(20.0);
            let normal = surface.normal_at(t * surface.arc_length(), angle);
            let radial = surface.radial_dir(angle);
            assert!(
                normal.dot(radial) > 0.0,
                "flared wall normal at t={t} should point away from the axis"
            );
        }
    }

    #[test]
    fn vertex_averaged_normals_at_a_kink_stay_between_the_two_segment_normals() {
        // A profile with a kink: straight up, then flared out. At the
        // vertex the normal should be a convex combination of the two
        // segment normals, and so also point away from the axis.
        let profile = vec![point(1.0, 0.0), point(1.0, 1.0), point(1.6, 2.0)];
        let surface =
            RevolutionSurface::try_new(WPoint3::zero(), WVec3::new(0.0, 0.0, 1.0), profile.clone())
                .expect("a kinked profile is a surface");

        let first_seg_len = segment_length(profile[0], profile[1]);
        let angle = Angle::degrees(0.0);
        let normal = surface.normal_at(first_seg_len, angle);
        let radial = surface.radial_dir(angle);
        assert!(
            normal.dot(radial) > 0.0,
            "the vertex normal at the kink should point away from the axis, got {normal:?}"
        );
    }

    #[test]
    fn point_at_spins_the_profile_around_the_axis() {
        let base = WPoint3::new(1.0, 2.0, 3.0);
        let axis = WVec3::new(0.0, 0.0, 1.0);
        let surface = RevolutionSurface::try_new(base, axis, straight_wall())
            .expect("a straight wall is a surface");

        let angle = Angle::degrees(0.0);
        let bottom = surface.point_at(0.0, angle);
        let bottom_offset = bottom - base;
        assert!(
            (bottom_offset.dot(axis) - 0.0).abs() < 1.0e-9,
            "height at t=0"
        );
        assert!(
            ((bottom_offset - axis * bottom_offset.dot(axis)).length() - 1.0).abs() < 1.0e-9,
            "radius at t=0"
        );

        let top = surface.point_at(surface.arc_length(), angle);
        let top_offset = top - base;
        assert!(
            (top_offset.dot(axis) - 1.0).abs() < 1.0e-9,
            "height at t=arc_length"
        );
        assert!(
            ((top_offset - axis * top_offset.dot(axis)).length() - 1.0).abs() < 1.0e-9,
            "radius at t=arc_length"
        );
    }

    #[test]
    fn normal_at_point_recovers_the_same_normal_as_normal_at() {
        let surface = RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            vec![point(0.5, 0.0), point(1.0, 1.0), point(0.8, 2.0)],
        )
        .expect("a flared then tapered wall is a surface");

        for t_fraction in [0.1, 0.4, 0.6, 0.9] {
            let t = t_fraction * surface.arc_length();
            let angle = Angle::degrees(65.0);
            let expected = surface.normal_at(t, angle);
            let point = surface.point_at(t, angle);
            let recovered = surface.normal_at_point(point);
            assert!(
                (expected - recovered).length() < 1.0e-6,
                "expected {expected:?}, recovered {recovered:?}"
            );
        }
    }

    #[test]
    fn max_radius_is_the_widest_profile_point() {
        let surface = RevolutionSurface::try_new(
            WPoint3::zero(),
            WVec3::new(0.0, 0.0, 1.0),
            vec![point(0.2, 0.0), point(0.9, 0.5), point(0.4, 1.0)],
        )
        .expect("a profile with a bulge is a surface");
        assert_eq!(surface.max_radius(), 0.9);
    }
}
