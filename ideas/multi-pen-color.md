# Multi-pen color: layers, transmission, optical mixing

Multicolor plots built from what pens actually are — spot inks laid down in
registered passes — with materials that can *alter what is seen through
them* (Sean's cyan-glass concept).

## The three layers of multicolor

### 1. Pen layers (foundation — mostly done)

Every stroke already carries a `PenId` and `Rendering` groups by pen. What
remains is emission: one SVG layer per pen with plotter-friendly naming
(AxiDraw/Inkscape convention `1-black`, `2-cyan`), so a plot becomes N
registered passes with pen swaps between.

### 2. Transmissive materials (the cyan-glass effect)

A pane of glass should not occlude — it should *transform* whatever is
drawn behind it. The mechanic falls out of the existing architecture:
visibility is a per-sample ray query, and (post fix-round) `Scene::visible`
already reasons about the ordered hits strictly between a point and the
eye. Replace its boolean with a closed sum:

```
Occlusion = Clear
          | Blocked
          | Through(Vec<TransmissiveHit>)   // ordered media stack, near to far
```

A material's transmission policy then transforms strokes sampled behind
it:

- **re-pen**: strokes behind the cyan pane plot with the cyan pen — the
  glass literally recolors its world.
- **tone shift**: transmission multiplies tone (dimmer behind glass ->
  sparser hatching), stacking per pane.
- **character**: frosted glass adds positional jitter to samples; old
  glass gets a refraction wobble; a screen door gets a dash pattern.

Light interacts the same way: shadow rays that hit transmissive surfaces
attenuate instead of blocking, so glass casts pale shadows and a cyan
skylight literally shifts the hatching density (and pen) of the floor
below it.

### 3. Optical color mixing (the far shore)

Pens are spot colors; blends happen in the eye. Interleaved cross-hatch
fields of two inks read as their optical mix at viewing distance (classic
engraving technique). With calibrated pens
([pen-tone-calibration.md](pen-tone-calibration.md) measures each ink's
tone curve; a color card measures hue), a material could specify a target
*color*, compiled into interleaved multi-pen hatching — stroke-space color
separation, CMYK-like but built from strokes instead of dots. Nobody has
this.

## Build order

1. Per-pen SVG layer emission (small; unlocks real multicolor plots
   immediately with per-material pens).
2. Transmission in the occlusion query + re-pen/tone policies (medium; the
   `visible()` seam is ready).
3. Attenuating shadow rays through transmissive materials (small, big
   visual payoff).
4. Refraction/frost stroke character (small, delightful).
5. Calibrated optical mixing (large; pairs with the calibration card).

## Open questions

- Where re-penning composes: two stacked panes with different tints — does
  the nearest pane win, or do policies compose (probably compose, nearest
  last)?
- Overdraw at pen boundaries: when a hatch run crosses from behind glass
  to open air its pen changes mid-run — split the run at the media
  boundary (the slicing machinery makes this natural).
- Whether tone thresholds should be per-pen once calibration lands (a cyan
  pen's "dark" differs from black's).
