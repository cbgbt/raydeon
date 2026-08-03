# Pen-and-paper tone calibration

Make raydeon the first line renderer that knows what its ink actually looks
like: a closed loop from plotted output back into the tone model.

## Problem

The tonal system is guesswork dressed in types. `ToneWhite`, the
`ToneThreshold` bands, hatch spacing — all tuned by eyeballing a screen. The
real output is a specific pen on a specific paper, and every pen/paper/speed
combination has a different tone response: how dark a 45-degree pass at 2mm
spacing actually reads, how much a second cross direction darkens it, and
where added density stops reading darker and floods into mud.

## Sketch

1. **Calibration card.** raydeon emits an SVG card per pen: a grid of
   swatches sweeping spacing x pass count x angle, with registration marks.
2. **Measure.** Plot the card once per pen/paper combo, photograph or scan
   it. A small pyraydeon tool finds the swatches via the registration marks
   and measures mean luminance per swatch, then fits a monotone curve:
   *stroke coverage -> measured tone on paper*.
3. **Invert.** The engine inverts the curve. A `HatchStyle` stops meaning
   "spacing 0.22 because it looked right" and starts meaning "hit this
   5-step gray ramp", compiled per-pen into the spacings and band
   thresholds that produce those grays on the measured setup.

## Why it fits raydeon

Tone already flows through one choke point: `Scene::tone_for_hit`
normalization and the `ToneThreshold` comparisons in the hatch engine. A
measured transfer function slots between illumination and the keep decision
without touching anything else.

## Compounding effects

- Multi-pen renders become tonally matched across different inks
  (see [multi-pen-color.md](multi-pen-color.md)).
- When transparency lands, overlap darkening between layered passes is a
  measurable curve instead of a guess.
- Different plotters/speeds stop changing how renders look: recalibrate,
  keep the scene.

## Open questions

- Curve model: monotone spline vs piecewise linear; how many swatches
  suffice.
- Camera-photo normalization (white balance, vignetting) vs requiring a
  flatbed scan.
- Where the fitted curves live (TOML per pen/paper profile, path via CLI,
  per Sean's config doctrine).
