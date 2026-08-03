"""Demonstrates world-space hatching: native shapes which shade themselves.

Both shapes draw their own outlines and offer their surfaces up to be
hatched. The light-flow style lays strokes along the light across the
sphere; the ground takes a stochastic style with a pen of its own.
"""

import numpy as np
import svg

from pyraydeon import (
    Camera,
    CameraOptions,
    HatchStyle,
    Material,
    PenId,
    Point3,
    PointLight,
    Quad,
    Scene,
    Sphere,
    Stroke,
    Vec3,
)

LIGHT = Point3(4, 3, 10)


sphere = Sphere(Point3(0, 0, 0), 1.0).with_material(
    Material(
        diffuse=3.0,
        specular=3.0,
        shininess=3.0,
        pen=PenId(1),
        hatch=HatchStyle.light_flow(source=LIGHT, spacing=0.11),
    )
)

ground = Quad(
    Point3(-4, -2, -4),
    np.array([[0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]),
    np.array([8.0, 8.0]),
).with_material(
    Material(
        diffuse=2.0,
        specular=0.0,
        shininess=1.0,
        hatch=HatchStyle.stochastic(spacing=0.3),
    )
)

scene = Scene(
    geometry=[sphere, ground],
    lights=[PointLight(LIGHT, 3.6, 2.0, 0.15, 0.4, 0.11)],
    ambient_light=0.13,
)

eye = Point3(0, 0, 0)
focus = Point3(0, 0, -1)
up = Vec3(0, 1, 0)

fovy = 50.0
width = 1024
height = 1024
znear = 0.1
zfar = 100.0

render_opts = CameraOptions(pen_px_size=4.0)

assert render_opts.hatch_slice_forgiveness == 1

cam = (
    Camera()
    .look_at(eye, focus, up)
    .perspective(fovy, width, height, znear, zfar)
    .render_options(render_opts)
)
cam.translate((0, 0, 5))

strokes: list[Stroke] = scene.render(cam, seed=5)

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
svg_lines = [
    svg.Line(
        x1=f"{stroke.p1[0]}",
        y1=f"{stroke.p1[1]}",
        x2=f"{stroke.p2[0]}",
        y2=f"{stroke.p2[1]}",
        stroke_width="0.7mm",
        stroke="black",
    )
    for stroke in strokes
]
line_group = svg.G(transform=f"translate(0, {height}) scale(1, -1)", elements=svg_lines)
canvas.elements = [backing_rect, line_group]


print(canvas)
