"""Demonstrates custom object and numpy support

This example is really slow, and a good case for finding more ways to express
geometry as the sum of native parts.
"""

import numpy as np
import svg

from pyraydeon import (
    AABB3,
    Camera,
    Geometry,
    HitData,
    LineSegment3D,
    Scene,
    CollisionGeometry,
    Plane,
)


class Quad(CollisionGeometry):
    def __init__(self, vertices):
        self.vertices = vertices
        self.plane = self.compute_plane(vertices)

    def compute_plane(self, points):
        p1, p2, p3 = points[:3]
        normal = np.cross(p2 - p1, p3 - p1)
        normal /= np.linalg.norm(normal)
        return Plane(p1, normal)

    def is_point_in_face(self, point):
        edge1 = self.vertices[1] - self.vertices[0]
        edge2 = self.vertices[3] - self.vertices[0]
        v = point - self.vertices[0]
        u1 = np.dot(v, edge1) / np.dot(edge1, edge1)
        u2 = np.dot(v, edge2) / np.dot(edge2, edge2)
        return 0 <= u1 <= 1 and 0 <= u2 <= 1

    def hit_by(self, ray) -> HitData | None:
        intersection = self.plane.hit_by(ray)
        if intersection is not None and self.is_point_in_face(intersection.hit_point):
            return intersection

    def bounding_box(self):
        my_min = np.minimum.reduce(self.vertices)
        my_max = np.maximum.reduce(self.vertices)
        return AABB3(my_min, my_max)


class RectPrism(Geometry):
    def __init__(
        self,
        origin: np.ndarray,
        right: np.ndarray,
        width: float,
        up: np.ndarray,
        height: float,
        depth: float,
    ):
        up = up / np.linalg.norm(up)
        right = right / np.linalg.norm(right)
        fwd = np.cross(up, right)
        fwd = fwd / np.linalg.norm(fwd)

        self.origin = origin
        self.right = right
        self.up = up
        self.fwd = fwd

        self.width = width
        self.height = height
        self.depth = depth

        self.vertices = np.array(
            [
                origin,
                origin + right * width,
                origin + right * width + fwd * depth,
                origin + fwd * depth,
                origin + up * height,
                origin + right * width + up * height,
                origin + right * width + fwd * depth + up * height,
                origin + fwd * depth + up * height,
            ]
        )

        # Make edges stand out slightly so as to not be intersected by their own faces
        origin = origin - (right * 0.0015) - (up * 0.0015) - (fwd * 0.0015)
        width = width + 0.003
        height = height + 0.003
        depth = depth + 0.003
        self.path_vertices = np.array(
            [
                origin,
                origin + right * width,
                origin + right * width + fwd * depth,
                origin + fwd * depth,
                origin + up * height,
                origin + right * width + up * height,
                origin + right * width + fwd * depth + up * height,
                origin + fwd * depth + up * height,
            ]
        )

        self.faces = [
            [0, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ]

    def __repr__(self):
        return f"RectPrism(basis='[{self.right}, {self.up}, {self.fwd}]', dims='[{self.width}, {self.height}, {self.depth}]')"

    def collision_geometry(self):
        return [Quad(self.vertices[face]) for face in self.faces]

    def paths(self, cam):
        edges = set(
            [
                tuple(sorted((face[i], face[(i + 1) % len(face)])))
                for face in self.faces
                for i in range(len(face))
            ]
        )
        paths = [
            LineSegment3D(self.path_vertices[edge[0]], self.path_vertices[edge[1]])
            for edge in edges
        ]
        return paths


up = np.array([-1.0, 1.0, 0.0])
up = up / np.linalg.norm(up)
right = np.array([1.0, 1.0, 0.0])
right = right / np.linalg.norm(right)


r = RectPrism(
    origin=np.array([0.0, 0.0, 0.0]),
    right=right,
    width=1.0,
    up=up,
    height=1.0,
    depth=1.0,
)

scene = Scene(
    [
        RectPrism(
            origin=np.array([0.0, 0.0, 0.0]),
            right=right,
            width=1.0,
            up=up,
            height=1.0,
            depth=1.0,
        ),
        RectPrism(
            origin=np.array([0.0, 0.0, 1.25]),
            right=right,
            width=1.0,
            up=up,
            height=1.0,
            depth=1.0,
        ),
        RectPrism(
            origin=right * 1.1,
            right=right,
            width=1.0,
            up=up,
            height=1.0,
            depth=1.0,
        ),
    ]
)

eye = np.array([0.25, 3, 6])
focus = np.array([0, 0, 0])
up = np.array([0, 1, 0])

fovy = 60.0
width = 1024
height = 1024
znear = 0.1
zfar = 10.0

cam = Camera.look_at(eye, focus, up).perspective(fovy, width, height, znear, zfar)

paths = scene.render(cam)

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
