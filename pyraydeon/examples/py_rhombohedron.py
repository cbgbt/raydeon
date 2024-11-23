"""Demonstrates custom objects with native collision geometry"""

import numpy as np
import svg

from pyraydeon import (
    Camera,
    Geometry,
    LineSegment3D,
    Scene,
    Quad,
)


class Rhombohedron(Geometry):
    def __init__(self, origin, basis, dims):
        basis = basis / np.linalg.norm(basis, axis=1, keepdims=True)

        self.origin = origin
        self.basis = basis
        self.dims = dims

        combinations = np.array(np.meshgrid([0, 1], [0, 1], [0, 1])).T.reshape(-1, 3)
        scaled_combinations = combinations * dims

        # Transform the scaled combinations using the basis
        transformed_vertices = np.dot(scaled_combinations, basis)
        # Shift by the origin
        self.vertices = transformed_vertices + origin

        # Make edges stand out slightly so as to not be intersected by their own faces
        centroid = np.mean(self.vertices, axis=0)
        vert_move_dirs = self.vertices - centroid
        unit_move_dirs = vert_move_dirs / np.linalg.norm(
            vert_move_dirs, axis=0, keepdims=True
        )
        move_vectors = unit_move_dirs * 0.0015
        self.path_vertices = self.vertices + move_vectors

        self.faces = [
            [0, 1, 3, 2],  # Bottom face
            [4, 5, 7, 6],  # Top face
            [0, 1, 5, 4],  # Front face
            [2, 3, 7, 6],  # Back face
            [0, 2, 6, 4],  # Left face
            [1, 3, 7, 5],  # Right face
        ]
        self.quads = self.compute_quads()

    def __repr__(self):
        return f"Rhomboid(origin='{self.origin}', basis='{self.basis}', dims='{self.dims}')"

    def compute_quads(self):
        quads = []
        for face in self.faces:
            verts = self.vertices[face]
            origin = verts[0]
            basis = np.array(
                [
                    verts[1] - origin,
                    verts[3] - origin,
                ]
            )
            dims = np.linalg.norm(basis, axis=1)

            quads.append(Quad(origin, basis, dims))
        return quads

    def collision_geometry(self):
        return [geom for quad in self.quads for geom in quad.collision_geometry()]

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


scene = Scene(
    [
        Rhombohedron(
            origin=np.array([0.0, 0.0, 0.0]),
            basis=np.array(
                [
                    [0.9, 0.5, 0.0],
                    [-0.3, 1.0, 0.0],
                    [-0.5, 0.25, -0.7],
                ]
            ),
            dims=np.array([1.0, 1.0, 1.0]),
        ),
        Rhombohedron(
            origin=np.array([2.0, 0.0, -2.0]),
            basis=np.array(
                [
                    [-0.9, 0.5, 0.0],
                    [0.3, 1.0, 0.5],
                    [1.5, 0.25, -0.7],
                ]
            ),
            dims=np.array([1.0, 1.0, 1.0]),
        ),
    ]
)

eye = np.array([0, 0.4, 5])
focus = np.array([0, 0.4, 0])
up = np.array([0, 1, 0])

fovy = 60.0
width = 1024
height = 1024
znear = 0.1
zfar = 20.0

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
