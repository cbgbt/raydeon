"""Demonstrates the native `Lathe` shape and a from-Python revolution surface.

The vase is `pyraydeon.Lathe`: a wheel-thrown profile, always spun around
+Z, hatched and given tone-and-silhouette contours through the new
`ContourStyle` kwarg. It stands on a `Plinth` — a shape written entirely in
Python which triangulates its own collision bands (the same way the native
`Lathe` builds its own) but offers up an exact `RevolutionSurface` for
hatching, tilted off +Z to exercise the general axis a native `Lathe`
cannot express. Its contours come from `ContourStyle.band_edges`, matching
its hatch style's own band thresholds.
"""

import numpy as np
import svg

from pyraydeon import (
    AABB3,
    Camera,
    CollisionGeometry,
    ContourStyle,
    Geometry,
    HatchStyle,
    HitData,
    Lathe,
    LineSegment3D,
    Material,
    PenId,
    PointLight,
    Quad,
    Ray,
    RevolutionSurface,
    Scene,
    Stroke,
    StrokeKind,
)

# Distinct stroke widths per `StrokeKind`, matching the rust examples
# (storefront's `write_svg`): outlines heaviest, hatching lighter, contours
# lightest — so the three concepts read apart on paper, not just in code.
# `StrokeKind` supports `==` but not hashing, so this is a list of pairs
# rather than a dict.
STROKE_WIDTHS = [
    (StrokeKind.Outline, "1.1mm"),
    (StrokeKind.Hatch, "0.7mm"),
    (StrokeKind.Contour, "0.45mm"),
]

LIGHT = np.array([-5.0, -6.0, 9.0])


class PyTriangle(CollisionGeometry):
    """A hand-rolled triangle collider (Möller-Trumbore intersection).

    pyraydeon exposes no bare-`CollisionGeometry` triangle to Python — `Tri`
    is a drawable `Geometry`, built for shapes whole — so the plinth's
    triangulated bands collide with a small implementation of the same
    ray-intersection algorithm the Rhombohedron example's `PyQuad` uses for
    its own faces.
    """

    def __init__(self, v0: np.ndarray, v1: np.ndarray, v2: np.ndarray) -> None:
        self.v0 = v0
        self.v1 = v1
        self.v2 = v2

    def hit_by(self, ray: Ray) -> HitData | None:
        edge1 = self.v1 - self.v0
        edge2 = self.v2 - self.v0
        direction = ray.dir / np.linalg.norm(ray.dir)

        pvec = np.cross(direction, edge2)
        det = np.dot(edge1, pvec)
        if abs(det) < 1e-9:
            return None
        inv_det = 1.0 / det

        tvec = ray.point - self.v0
        u = np.dot(tvec, pvec) * inv_det
        if u < 0.0 or u > 1.0:
            return None

        qvec = np.cross(tvec, edge1)
        v = np.dot(direction, qvec) * inv_det
        if v < 0.0 or u + v > 1.0:
            return None

        t = np.dot(edge2, qvec) * inv_det
        if t <= 1e-9:
            return None

        hit_point = ray.point + direction * t
        normal = np.cross(edge1, edge2)
        normal = normal / np.linalg.norm(normal)
        return HitData(hit_point, t, normal)

    def bounding_box(self) -> AABB3:
        lo = np.minimum.reduce([self.v0, self.v1, self.v2])
        hi = np.maximum.reduce([self.v0, self.v1, self.v2])
        return AABB3(lo, hi)


class Plinth(Geometry):
    """A tapered stand, spun around a slightly tilted axis.

    Its triangulated bands (built the same way the native `Lathe` builds its
    own collision geometry) collide and draw; `hatch_surfaces` offers up the
    exact `RevolutionSurface` those bands only approximate, so world-space
    hatching and contouring see the true curved surface rather than its
    facets.
    """

    SEGMENTS = 24
    RING_LIFT = 0.01

    def __init__(self, base: np.ndarray, axis: np.ndarray, profile: np.ndarray) -> None:
        self.base = np.asarray(base, dtype=float)
        self.axis = np.asarray(axis, dtype=float)
        self.axis = self.axis / np.linalg.norm(self.axis)
        self.profile = np.asarray(profile, dtype=float)

        seed = np.array([0.0, 0.0, 1.0])
        if abs(np.dot(seed, self.axis)) > 0.99:
            seed = np.array([1.0, 0.0, 0.0])
        self.u = np.cross(self.axis, seed)
        self.u = self.u / np.linalg.norm(self.u)
        self.v = np.cross(self.axis, self.u)

    def ring_point(self, radius: float, height: float, ndx: int) -> np.ndarray:
        angle = (ndx % self.SEGMENTS) / self.SEGMENTS * 2.0 * np.pi
        radial = self.u * np.cos(angle) + self.v * np.sin(angle)
        return self.base + radial * radius + self.axis * height

    def collision_geometry(self) -> list[CollisionGeometry]:
        triangles: list[CollisionGeometry] = []
        for (r0, h0), (r1, h1) in zip(self.profile[:-1], self.profile[1:]):
            for ndx in range(self.SEGMENTS):
                a0 = self.ring_point(r0, h0, ndx)
                a1 = self.ring_point(r0, h0, ndx + 1)
                b0 = self.ring_point(r1, h1, ndx)
                b1 = self.ring_point(r1, h1, ndx + 1)
                triangles.append(PyTriangle(a0, a1, b0))
                triangles.append(PyTriangle(b0, a1, b1))
        return triangles

    def paths(self, cam: Camera) -> list[LineSegment3D]:
        segments = []
        for radius, height in self.profile:
            lifted = radius + self.RING_LIFT
            for ndx in range(self.SEGMENTS):
                p0 = self.ring_point(lifted, height, ndx)
                p1 = self.ring_point(lifted, height, ndx + 1)
                segments.append(LineSegment3D(p0, p1))
        return segments

    def hatch_surfaces(self) -> list[RevolutionSurface]:
        return [RevolutionSurface(self.base, self.axis, self.profile)]


vase_hatch = HatchStyle.tonal_crosshatch(spacing=0.1)
vase = Lathe(
    np.array([0.0, 0.0, 0.62]),
    np.array(
        [
            [0.5, 0.0],
            [0.62, 0.12],
            [0.5, 0.28],
            [0.78, 0.55],
            [0.9, 0.82],
            [0.72, 1.05],
            [0.42, 1.2],
            [0.34, 1.3],
            [0.5, 1.4],
        ]
    ),
).with_material(
    Material(
        diffuse=1.0,
        specular=0.4,
        shininess=10.0,
        pen=PenId(1),
        hatch=vase_hatch,
        contours=ContourStyle(tone=[0.3, 0.6], silhouette=True, resolution=0.05),
    )
)

plinth_hatch = HatchStyle.tonal_crosshatch(spacing=0.16)
plinth = Plinth(
    base=np.array([0.0, 0.0, 0.0]),
    axis=np.array([0.08, 0.0, 1.0]),
    profile=np.array([[0.95, 0.0], [0.95, 0.2], [0.66, 0.62]]),
).with_material(
    Material(
        diffuse=1.0,
        specular=0.15,
        shininess=3.0,
        pen=PenId(2),
        hatch=plinth_hatch,
        contours=ContourStyle.band_edges(plinth_hatch),
    )
)

ground = Quad(
    np.array([-3.5, -3.5, 0.0]),
    np.array([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
    np.array([7.0, 7.0]),
).with_material(
    Material(
        diffuse=1.0,
        specular=0.0,
        shininess=1.0,
        pen=PenId(0),
        hatch=HatchStyle.stochastic(spacing=0.28),
        contours=ContourStyle(tone=[0.45]),
    )
)

scene = Scene(
    geometry=[ground, plinth, vase],
    lights=[PointLight(LIGHT, 3.2, 1.4, 0.2, 0.05, 0.01)],
    ambient_light=0.12,
)

eye = np.array([4.6, -6.0, 3.2])
focus = np.array([0.0, 0.0, 1.0])
up = np.array([0.0, 0.0, 1.0])

fovy = 45.0
width = 1024
height = 1024
znear = 0.1
zfar = 30.0

cam = Camera().look_at(eye, focus, up).perspective(fovy, width, height, znear, zfar)

strokes: list[Stroke] = scene.render(cam)

canvas = svg.SVG(
    width="8in",
    height="8in",
    viewBox="0 0 1024 1024",
)
backing_rect = svg.Rect(
    x=0,
    y=0,
    width="100%",
    height="100%",
    fill="white",
)
kind_groups = [
    svg.G(
        elements=[
            svg.Line(
                x1=f"{stroke.p1[0]}",
                y1=f"{stroke.p1[1]}",
                x2=f"{stroke.p2[0]}",
                y2=f"{stroke.p2[1]}",
                stroke="black",
            )
            for stroke in strokes
            if stroke.kind == kind
        ],
        stroke_width=width,
    )
    for kind, width in STROKE_WIDTHS
]
line_group = svg.G(
    transform=f"translate(0, {height}) scale(1, -1)", elements=kind_groups
)
canvas.elements = [backing_rect, line_group]


print(canvas)
