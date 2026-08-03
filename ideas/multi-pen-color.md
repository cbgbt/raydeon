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

**What glass does to the world behind it is an open design space** (Sean,
2026-08-03: "i don't know if just cyan is enough or if its cyan hatching
with interesting properties or everything is cyan"). Three distinct models,
none obviously right from the armchair:

- **Re-pen**: everything behind the pane plots in the glass's pen. Boldest
  stylization; the palette dies at the pane (a brown branch behind the
  window turns cyan). Right for stained glass, wrong for a shopfront.
- **Overlay**: geometry behind keeps its own pens and hatching (perhaps
  lightened); the pane contributes its own marks — streaks, a reflection
  wedge, grazing-angle density. Closest to how artists draw glass, and
  nearly free: a glass material with its own `HatchStyle` already draws
  its own marks; it only also needs to not-occlude.
- **Filter**: behind-glass strokes keep their pens but get modulated —
  tone dimmed, runs dashed, samples jittered (frost/wobble/screen-door).

Likely answer: all three exist, chosen per material. To be settled by
looking, not deriving — see "Engine vs app" below.

Light interacts the same way regardless of model: shadow rays that hit
transmissive surfaces attenuate instead of blocking, so glass casts pale
shadows and a cyan skylight shifts the hatching of the floor below it.

### Engine vs app

The engine ships **facts**, never aesthetics:

1. The occlusion query returns the ordered media stack, not a boolean.
2. Strokes carry provenance: source pen plus the media they were seen
   through.
3. Shadow rays attenuate through transmissive materials.

**Policy starts app-side.** With media stacks on strokes, each model above
is a stroke post-transform an app can write in a screenful of code. The
way to choose between them is the hatch-lab method: a *glass lab* — the
tree behind a glazed wall, all three policies rendered side by side,
compared visually. Policies that win graduate into the engine as named
transmission styles, exactly as the validated hatch strategies became
`HatchStyle` presets. No cyan is hardcoded in the engine at any point.

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
