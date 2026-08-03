# Principled NPR line extraction over implicit surfaces

> "principled NPR line extraction over implicit surfaces — occluding
> contours plus suggestive contours plus curvature-driven hatching — so
> raydeon stops rendering wireframes and starts producing drawings."
>
> — suggested by another Claude instance

## Problem

Every raydeon shape hand-authors its own `paths()`: a cuboid draws its 12
edges, a quad its outline. That is wireframe rendering. Real drawings are
made of *extracted* lines — the lines an artist would choose — and those are
properties of the surface and the view, not a fixed edge list.

## Sketch

Add implicit surfaces (signed distance fields) as a geometry representation
and extract lines from them:

- **Occluding contours**: the loci where `normal . view = 0` — true
  silhouettes of smooth forms, found by root-tracing on the SDF. The
  sphere silhouette added in the hatch-lab work is the hand-derived special
  case; this generalizes it to arbitrary smooth surfaces.
- **Suggestive contours** (DeCarlo et al.): zero crossings of radial
  curvature where a contour *almost* forms — the lines that make a drawing
  read as drawn rather than traced. View-dependent, needs curvature of the
  field (second derivatives of the SDF; automatic with analytic SDFs).
- **Ridges and valleys**: view-independent creases of the curvature field;
  good for terrain, drapery, and architectural moldings.
- **Curvature-driven hatching**: hatch direction from principal curvature
  directions (Hertzmann/Zorin style cross fields) instead of a fixed
  face-plane angle — strokes that wrap forms the way an engraver's do.

## Why it fits raydeon

- SDFs compose (smooth blends, CSG) and are cheap to ray-march; the BVH and
  the sampling occlusion pipeline don't care where a `CollisionGeometry`
  hit comes from.
- The hatch engine's `HatchSurface` sum gains a third variant
  (`Implicit`) whose line generation samples direction fields instead of
  face-plane parallels; the tone-filter/clip pipeline is unchanged.
- Procedural modeling gets a huge lever: terrain, blobby vegetation,
  eroded stone — all natural as SDFs, all currently impossible as quads.

## Open questions

- Contour tracing robustness (loop closure, singular points) — the hard
  part of the literature.
- Suggestive contours need stable second derivatives: analytic SDF library
  vs finite differences with care.
- Chaining: extracted contours arrive as polylines; the current pipeline
  slices parent line segments — polyline parents want first-class support
  (also relevant to plot path optimization).
