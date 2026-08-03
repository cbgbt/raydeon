//! How a material asks to be shaded.
//!
//! A hatch style says which lines to draw and when to draw them; where those
//! lines go is the surface's business. The three variants are the strategies
//! which survived visual comparison, and the presets are their tuned
//! parameters.

use crate::WPoint3;
use euclid::Angle;
use nutype::nutype;

/// Distance between neighbouring hatch lines, in world units.
///
/// A non-positive spacing would step through a surface forever, so it is
/// parsed rather than trusted.
#[nutype(
    validate(finite, greater = 0.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct HatchSpacing(f64);

/// How readily a jittered pass keeps a sample: the keep probability at a
/// given tone is scaled by this, so `0.0` draws nothing and `1.0` draws the
/// full jittered density.
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct HatchCoverage(f64);

/// A tone cutoff: a pass draws where the surface tone falls below it.
#[nutype(
    validate(finite, greater_or_equal = 0.0, less_or_equal = 1.0),
    derive(Debug, Clone, Copy, PartialEq, PartialOrd)
)]
pub struct ToneThreshold(f64);

/// One band of an engraving: a direction and spacing which appear only where
/// the surface is darker than `threshold`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TonalPass {
    pub angle: Angle<f64>,
    pub spacing: HatchSpacing,
    pub threshold: ToneThreshold,
}

/// The shading strategy a material draws itself with.
#[derive(Debug, Clone, PartialEq)]
pub enum HatchStyle {
    /// Engraving bands: each pass adds lines wherever the tone falls below
    /// that pass's threshold, so darker regions accumulate directions.
    Tonal { passes: Vec<TonalPass> },
    /// A single direction whose keep probability rises as the tone falls,
    /// scattered by position-hashed jitter.
    Stochastic {
        angle: Angle<f64>,
        spacing: HatchSpacing,
        coverage: HatchCoverage,
    },
    /// Lines flow along the light direction projected onto each surface. A
    /// perpendicular cross pass appears only below `cross_threshold`.
    LightFlow {
        source: WPoint3,
        spacing: HatchSpacing,
        cross_spacing: HatchSpacing,
        cross_threshold: ToneThreshold,
        coverage: HatchCoverage,
    },
}

/// Ratio of the tight cross-hatch spacing to the base spacing.
const TIGHT_RATIO: f64 = 0.68;

impl HatchStyle {
    /// Four-direction engraving: two diagonals over the midtones, then
    /// horizontal and vertical passes tightening the darkest regions.
    ///
    /// On a sphere the first three passes become contour rings about the Z, X
    /// and Y axes; a fourth pass would only overdraw them.
    pub fn tonal_crosshatch(spacing: HatchSpacing) -> Self {
        let tight = tighter(spacing);
        HatchStyle::Tonal {
            passes: vec![
                pass(45.0, spacing, 0.78),
                pass(135.0, spacing, 0.55),
                pass(0.0, tight, 0.33),
                pass(90.0, tight, 0.16),
            ],
        }
    }

    /// A single diagonal direction thinning out towards the light.
    pub fn stochastic(spacing: HatchSpacing) -> Self {
        HatchStyle::Stochastic {
            angle: Angle::degrees(45.0),
            spacing,
            coverage: coverage(0.9),
        }
    }

    /// Strokes which follow the light across each surface, crossed only in
    /// deep shadow. `source` is where the light comes from; it is stated
    /// rather than read from the scene so a style stays self-contained.
    ///
    /// Tone is normalized by the scene's `tone_white`, so tune that to the
    /// scene's lighting before tuning the style.
    pub fn light_flow(source: WPoint3, spacing: HatchSpacing) -> Self {
        HatchStyle::LightFlow {
            source,
            spacing,
            cross_spacing: tighter(spacing),
            cross_threshold: threshold(0.2),
            coverage: coverage(0.95),
        }
    }

    /// The tone levels at which this style's appearance steps discretely:
    /// `Tonal`'s own pass thresholds, `LightFlow`'s single cross threshold, or
    /// nothing for `Stochastic`, which has no discrete band edges.
    pub fn band_thresholds(&self) -> Vec<ToneThreshold> {
        match self {
            HatchStyle::Tonal { passes } => passes.iter().map(|pass| pass.threshold).collect(),
            HatchStyle::Stochastic { .. } => Vec::new(),
            HatchStyle::LightFlow {
                cross_threshold, ..
            } => vec![*cross_threshold],
        }
    }
}

fn pass(degrees: f64, spacing: HatchSpacing, cutoff: f64) -> TonalPass {
    TonalPass {
        angle: Angle::degrees(degrees),
        spacing,
        threshold: threshold(cutoff),
    }
}

fn tighter(spacing: HatchSpacing) -> HatchSpacing {
    HatchSpacing::try_new(spacing.into_inner() * TIGHT_RATIO)
        .expect("a positive fraction of a valid spacing is a valid spacing")
}

fn coverage(value: f64) -> HatchCoverage {
    HatchCoverage::try_new(value).expect("preset coverage is a tone fraction")
}

fn threshold(value: f64) -> ToneThreshold {
    ToneThreshold::try_new(value).expect("preset threshold is a tone fraction")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spacing() -> HatchSpacing {
        HatchSpacing::try_new(0.22).expect("0.22 is a valid spacing")
    }

    #[test]
    fn spacing_rejects_the_values_which_would_never_terminate() {
        assert!(HatchSpacing::try_new(0.0).is_err());
        assert!(HatchSpacing::try_new(-1.0).is_err());
        assert!(HatchSpacing::try_new(f64::NAN).is_err());
        assert!(HatchSpacing::try_new(f64::INFINITY).is_err());
    }

    #[test]
    fn tone_fractions_stay_within_the_unit_interval() {
        assert!(HatchCoverage::try_new(1.5).is_err());
        assert!(HatchCoverage::try_new(-0.1).is_err());
        assert!(ToneThreshold::try_new(1.5).is_err());
        assert!(ToneThreshold::try_new(f64::NAN).is_err());
        assert!(ToneThreshold::try_new(0.0).is_ok());
        assert!(ToneThreshold::try_new(1.0).is_ok());
    }

    #[test]
    fn the_tonal_preset_darkens_in_four_stages() {
        let HatchStyle::Tonal { passes } = HatchStyle::tonal_crosshatch(spacing()) else {
            panic!("the tonal preset is a tonal style");
        };
        assert_eq!(passes.len(), 4);
        for pair in passes.windows(2) {
            assert!(
                pair[1].threshold < pair[0].threshold,
                "later passes must apply to darker tones"
            );
        }
    }

    #[test]
    fn band_thresholds_reports_each_style_familys_step_edges() {
        let HatchStyle::Tonal { passes } = HatchStyle::tonal_crosshatch(spacing()) else {
            panic!("the tonal preset is a tonal style");
        };
        let expected: Vec<ToneThreshold> = passes.iter().map(|pass| pass.threshold).collect();
        assert_eq!(
            HatchStyle::tonal_crosshatch(spacing()).band_thresholds(),
            expected
        );

        assert!(HatchStyle::stochastic(spacing())
            .band_thresholds()
            .is_empty());

        let HatchStyle::LightFlow {
            cross_threshold, ..
        } = HatchStyle::light_flow(WPoint3::new(0.0, 0.0, 5.0), spacing())
        else {
            panic!("the light flow preset is a light flow style");
        };
        assert_eq!(
            HatchStyle::light_flow(WPoint3::new(0.0, 0.0, 5.0), spacing()).band_thresholds(),
            vec![cross_threshold]
        );
    }

    #[test]
    fn the_light_flow_preset_crosses_only_in_shadow() {
        let HatchStyle::LightFlow {
            cross_spacing,
            cross_threshold,
            ..
        } = HatchStyle::light_flow(WPoint3::new(0.0, 0.0, 5.0), spacing())
        else {
            panic!("the light flow preset is a light flow style");
        };
        assert!(cross_spacing < spacing());
        assert!(cross_threshold < threshold(0.5));
    }
}
