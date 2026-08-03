# Shadow outlines

Draw the *boundary* of a shadow as a stroke, not just its hatched fill —
Sean's observation (2026-08-03) that the darkest hatched areas are hard to
grok, and that outlining shadows "seems hard." It is more tractable than it
looks.

## Problem

Dense cross-hatch regions (the shadow side of the blade sign, deep alcove
corners) read as texture without form: the eye has no edge to anchor the
shadow's shape. Classical pen artists solve this by outlining the shadow
terminator and cast-shadow boundary, then filling more loosely.

## Sketch

The engine already samples tone anywhere on a surface. Shadow outlines are
iso-tone contours of that field:

1. For each hatchable surface, sample tone on a regular grid in face
   coordinates (the same `tone_for_hit` the hatch filter uses).
2. Run marching squares at a chosen iso value (e.g. the material's first
   `ToneThreshold`) to extract contour polylines — the lit/shadow boundary,
   including cast-shadow edges from other geometry.
3. Chain the cell crossings into polylines, lift them off the surface like
   hatch lines, and feed them through the existing clip-and-project
   pipeline as outline-kind strokes.

Because thresholds already delimit tonal bands, each `TonalPass` boundary
can optionally carry its outline: an engraving whose bands are edged. At
the extreme, outlines *without* fill give a pure contour drawing — shadow
shapes drawn as closed curves, hatching omitted entirely.

## Why it fits raydeon

Everything except the marching squares exists: face frames, tone sampling,
surface lift, occlusion clip. The new work is 2D contour extraction and
polyline chaining, entirely inside the hatch engine; spheres need the same
grid in their two ring parameters. A `HatchStyle` option (per pass or per
material) chooses fill, outline, or both.

## Open questions

- Grid resolution vs contour smoothness (and cost: one tone sample per
  cell corner; the hatch filter already pays comparable counts).
- Chaining across cell boundaries robustly (standard marching-squares
  ambiguity cases).
- Whether outlines should suppress hatch lines that would touch them
  (a clearance band keeps the boundary crisp on paper).
