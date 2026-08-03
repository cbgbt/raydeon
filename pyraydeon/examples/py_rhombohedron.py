"""Demonstrates custom objects with native collision geometry and hatching.

A shape written in Python draws itself, collides with native quads, and opts
in to world-space hatching by offering a planar surface per face.
"""

import numpy as np
import svg

from pyraydeon import (
    Camera,
    CollisionGeometry,
    Geometry,
    HatchStyle,
    LineSegment3D,
    Material,
    PenId,
    PlanarSurface,
    PointLight,
    Quad,
    Scene,
    Stroke,
)

LIGHT = np.array([4.0, 6.0, 8.0])


class Rhombohedron(Geometry):
    def __init__(self, origin: np.ndarray, basis: np.ndarray, dims: np.ndarray) -> None:
        basis = basis / np.linalg.norm(basis, axis=1, keepdims=True)

        self.origin = origin
        self.basis = basis
        self.dims = dims

        self.faces = [
            [0, 1, 3, 2],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 3, 7, 5],
        ]

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

        self.quads = self.compute_quads()

    def __repr__(self) -> str:
        return f"Rhomboid(origin='{self.origin}', basis='{self.basis}', dims='{self.dims}')"

    def compute_quads(self) -> list[Quad]:
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

    def collision_geometry(self) -> list[CollisionGeometry]:
        return [geom for quad in self.quads for geom in quad.collision_geometry()]

    def paths(self, cam: Camera) -> list[LineSegment3D]:
        edges = {
            tuple(sorted((face[i], face[(i + 1) % len(face)])))
            for face in self.faces
            for i in range(len(face))
        }
        paths = [
            LineSegment3D(self.path_vertices[edge[0]], self.path_vertices[edge[1]])
            for edge in edges
        ]
        return paths

    def hatch_surfaces(self) -> list[PlanarSurface]:
        """Each face, in the (possibly skewed) frame its own edges define.

        The faces are rhombi, not rectangles, but `PlanarSurface` now
        Gram-Schmidt orthonormalizes whatever basis it is given and
        re-expresses the outline exactly — so the rhombus edge basis is
        passed straight through, with no hand-rolled orthonormalization here.
        """
        unit_square = np.array([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]])
        centroid = np.mean(self.vertices, axis=0)
        surfaces = []
        for face in self.faces:
            verts = self.vertices[face]
            origin = verts[0]
            b0 = verts[1] - origin
            b1 = verts[3] - origin

            normal = np.cross(b0, b1)
            if np.dot(normal, origin - centroid) < 0:
                b0, b1 = b1, b0
                outline = unit_square[:, ::-1]
            else:
                outline = unit_square

            surfaces.append(PlanarSurface(origin, np.array([b0, b1]), outline))
        return surfaces


def hatched(pen: int) -> Material:
    return Material(
        diffuse=1.0,
        specular=0.2,
        shininess=4.0,
        pen=PenId(pen),
        hatch=HatchStyle.tonal_crosshatch(spacing=0.25),
    )


scene = Scene(
    geometry=[
        Rhombohedron(
            origin=np.array([0.0, 0.0, 0.2]),
            basis=np.array(
                [
                    [0.45, 0.2, -0.3],
                    [-0.3, 1.0, 0.0],
                    [-0.5, 0.0, -0.2],
                ]
            ),
            dims=np.array([2.0, 0.5, 1.0]),
        ).with_material(hatched(0)),
        Rhombohedron(
            origin=np.array([1.0, 0.0, -2.0]),
            basis=np.array(
                [
                    [-0.9, 0.5, 0.0],
                    [0.3, 1.0, 0.5],
                    [1.5, 0.25, -0.7],
                ]
            ),
            dims=np.array([1.0, 1.0, 0.8]),
        ).with_material(hatched(1)),
    ],
    lights=[PointLight(LIGHT, 5.0, 0.0, 1.0, 0.05, 0.01)],
    ambient_light=0.1,
)

eye = np.array([0, -0.5, 5])
focus = np.array([0, 0.4, 0])
up = np.array([0, 1, 0])

fovy = 60.0
width = 1024
height = 1024
znear = 0.1
zfar = 20.0

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
