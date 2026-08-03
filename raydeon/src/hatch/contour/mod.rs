//! Contour vocabulary: which families of iso-lines a material draws over its
//! hatch surfaces, and how densely the underlying scalar field is sampled.
//!
//! Nothing here samples a field or walks a grid — that is the contour
//! engine's job, added alongside the code that actually emits `Contour`
//! strokes. This module only states, and validates, what a material is
//! asking for.

use super::lines::SAMPLE_LEN;
use super::style::{HatchStyle, ToneThreshold};
use bon::Builder;
use nutype::nutype;

/// World-space distance between contour field samples.
///
/// A non-positive or infinite resolution would step through a grid forever
/// or never at all, so it is parsed rather than trusted. The default mints
/// from `lines::SAMPLE_LEN`, the same const hatching's tone filter samples
/// at, so retuning "how band-limited is the tone field" cannot silently
/// diverge the two callers.
#[nutype(
    validate(finite, greater = 0.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct ContourResolution(f64);

/// One family of iso-lines a material draws: a scalar field and its level.
#[derive(Debug, Clone, PartialEq)]
pub enum ContourField {
    /// Iso-line of the lit tone at this level — shadow boundaries, including
    /// cast-shadow edges, exactly where a hatch pass at this threshold starts.
    Tone(ToneThreshold),
    /// Zero contour of normal·view: the occluding contour on curved surfaces.
    Silhouette,
}

/// Which contours a material draws, and how densely their fields are
/// sampled.
#[derive(Debug, Clone, PartialEq, Builder)]
#[builder(start_fn(name = new))]
#[non_exhaustive]
pub struct ContourStyle {
    /// Deduped at construction (exact equality, order-preserving: first
    /// occurrence wins). Marching is deterministic, so a repeated field would
    /// otherwise draw the same ink twice.
    #[builder(with = |fields: impl Into<Vec<ContourField>>| dedupe(fields.into()))]
    fields: Vec<ContourField>,
    #[builder(default = default_resolution())]
    resolution: ContourResolution,
}

impl ContourStyle {
    pub fn fields(&self) -> &[ContourField] {
        &self.fields
    }

    pub fn resolution(&self) -> ContourResolution {
        self.resolution
    }

    /// One tone contour: the shadow boundary at `threshold`.
    pub fn shadow(threshold: ToneThreshold) -> Self {
        ContourStyle::new()
            .fields(vec![ContourField::Tone(threshold)])
            .build()
    }

    /// The occluding contour of a curved surface.
    pub fn silhouette() -> Self {
        ContourStyle::new()
            .fields(vec![ContourField::Silhouette])
            .build()
    }

    /// One tone contour per band edge `style` steps at — the convenience for
    /// matching a hatch style's own discrete tones.
    pub fn band_edges(style: &HatchStyle) -> Self {
        let fields: Vec<ContourField> = style
            .band_thresholds()
            .into_iter()
            .map(ContourField::Tone)
            .collect();
        ContourStyle::new().fields(fields).build()
    }
}

fn default_resolution() -> ContourResolution {
    ContourResolution::try_new(SAMPLE_LEN)
        .expect("SAMPLE_LEN is a positive finite world-unit length")
}

/// Keeps only the first occurrence of each exact value, preserving order.
fn dedupe(fields: Vec<ContourField>) -> Vec<ContourField> {
    let mut deduped: Vec<ContourField> = Vec::with_capacity(fields.len());
    for field in fields {
        if !deduped.contains(&field) {
            deduped.push(field);
        }
    }
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hatch::style::{HatchSpacing, TonalPass};
    use euclid::Angle;

    fn threshold(value: f64) -> ToneThreshold {
        ToneThreshold::try_new(value).expect("a tone fraction is a valid threshold")
    }

    #[test]
    fn resolution_rejects_the_values_which_would_never_terminate() {
        assert!(ContourResolution::try_new(0.0).is_err());
        assert!(ContourResolution::try_new(-1.0).is_err());
        assert!(ContourResolution::try_new(f64::NAN).is_err());
        assert!(ContourResolution::try_new(f64::INFINITY).is_err());
        assert!(ContourResolution::try_new(0.16).is_ok());
    }

    #[test]
    fn resolution_defaults_to_the_shared_sample_length() {
        let style = ContourStyle::new()
            .fields(vec![ContourField::Silhouette])
            .build();
        assert_eq!(
            style.resolution(),
            ContourResolution::try_new(SAMPLE_LEN).expect("SAMPLE_LEN is valid")
        );
    }

    #[test]
    fn shadow_preset_carries_one_tone_field() {
        let style = ContourStyle::shadow(threshold(0.4));
        assert_eq!(style.fields(), &[ContourField::Tone(threshold(0.4))]);
    }

    #[test]
    fn silhouette_preset_carries_one_silhouette_field() {
        let style = ContourStyle::silhouette();
        assert_eq!(style.fields(), &[ContourField::Silhouette]);
    }

    #[test]
    fn band_edges_mirrors_a_tonal_styles_pass_thresholds() {
        let spacing = HatchSpacing::try_new(0.3).expect("0.3 is a valid spacing");
        let hatch = HatchStyle::Tonal {
            passes: vec![
                TonalPass {
                    angle: Angle::degrees(0.0),
                    spacing,
                    threshold: threshold(0.7),
                },
                TonalPass {
                    angle: Angle::degrees(90.0),
                    spacing,
                    threshold: threshold(0.3),
                },
            ],
        };

        let style = ContourStyle::band_edges(&hatch);
        assert_eq!(
            style.fields(),
            &[
                ContourField::Tone(threshold(0.7)),
                ContourField::Tone(threshold(0.3))
            ]
        );
    }

    #[test]
    fn duplicate_fields_collapse_to_one_first_occurrence_wins() {
        let style = ContourStyle::new()
            .fields(vec![
                ContourField::Tone(threshold(0.5)),
                ContourField::Silhouette,
                ContourField::Tone(threshold(0.5)),
            ])
            .build();

        assert_eq!(
            style.fields(),
            &[ContourField::Tone(threshold(0.5)), ContourField::Silhouette]
        );
    }

    #[test]
    fn band_edges_on_a_tonal_style_with_repeated_thresholds_dedupes() {
        let spacing = HatchSpacing::try_new(0.3).expect("0.3 is a valid spacing");
        let hatch = HatchStyle::Tonal {
            passes: vec![
                TonalPass {
                    angle: Angle::degrees(0.0),
                    spacing,
                    threshold: threshold(0.5),
                },
                TonalPass {
                    angle: Angle::degrees(90.0),
                    spacing,
                    threshold: threshold(0.5),
                },
            ],
        };

        let style = ContourStyle::band_edges(&hatch);
        assert_eq!(style.fields(), &[ContourField::Tone(threshold(0.5))]);
    }
}
