import math
import svg
import numpy as np

from pyraydeon import (
    Camera,
    Point3,
    Scene,
    Sphere,
    Vec3,
    Geometry,
    Material,
    PointLight,
    Plane,
    LineSegment3D,
)


class PySphere(Geometry):
    def __init__(self, point, radius, material=None):
        if material is not None:
            self._material = material

        self.sphere = Sphere(point, radius)

    @property
    def material(self):
        return self._material

    def collision_geometry(self):
        return [self.sphere]

    def paths(self, cam):
        hyp = np.linalg.norm(self.sphere.center - cam.eye)
        opp = self.sphere.radius

        theta = math.asin(opp / hyp)
        adj = opp / math.tan(theta)
        d = math.cos(theta) * adj
        r = math.sin(theta) * adj

        w = self.sphere.center - cam.eye
        w = w / np.linalg.norm(w)

        u = np.cross(w, cam.up)
        u = u / np.linalg.norm(u)

        v = np.cross(w, u)
        v = v / np.linalg.norm(v)

        points = []
        c = cam.eye + d * w
        for i in range(0, 180):
            a = math.radians(float(i))
            p = c
            p = p + u * (math.cos(a) * r)
            p = p + v * (math.sin(a) * r)

            push = p - self.sphere.center
            push = push / np.linalg.norm(push)

            p += push * 0.00015
            points.append(p)

        paths = []
        for i in range(0, len(points) - 1):
            paths.append(LineSegment3D(points[i], points[(i + 1)]))

        return paths


class PyPlane(Geometry):
    def __init__(self, point, normal, material=None):
        if material is not None:
            self._material = material

        self.plane = Plane(point, normal)

    @property
    def material(self):
        return self._material

    def collision_geometry(self):
        return [self.plane]

    def paths(self, cam):
        return []


scene = Scene(
    geometry=[
        PySphere(Point3(0, 0, 0), 1.0, Material(3.0, 3.0, 3)),
        PyPlane(Point3(0, -2, 0), Vec3(0, 1, 0), Material(9000.0, 3.0, 3)),
    ],
    lights=[PointLight((4, 3, 10), 4.0, 2.0, 0.15, 0.4, 0.11)],
)


eye = Point3(0, 0, 5)
focus = Vec3(0, 0, 0)
up = Vec3(0, 1, 0)

fovy = 50.0
width = 1024
height = 1024
znear = 0.1
zfar = 100.0

cam = Camera().look_at(eye, focus, up).perspective(fovy, width, height, znear, zfar)

paths = scene.render_with_lighting(cam, seed=5)

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
        x1=f"{path.p1[0]}",
        y1=f"{path.p1[1]}",
        x2=f"{path.p2[0]}",
        y2=f"{path.p2[1]}",
        stroke_width="0.7mm",
        stroke="black",
    )
    for path in paths
]
line_group = svg.G(transform=f"translate(0, {height}) scale(1, -1)", elements=svg_lines)
canvas.elements = [backing_rect, line_group]


print(canvas)
