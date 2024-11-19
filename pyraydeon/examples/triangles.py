import svg

from pyraydeon import Camera, Point3, Scene, Tri, Vec3, Geometry


class CustomTriangle(Geometry):
    def __init__(self, p1, p2, p3):
        self.tri = Tri(p1, p2, p3)

    def hit_by(self, ray):
        return self.tri.hit_by(ray)

    def paths(self, cam):
        return self.tri.paths(cam)

    def bounding_box(self):
        return self.tri.bounding_box()


scene = Scene(
    [
        CustomTriangle(
            Point3(0, 0, 0),
            Point3(0, 0, 1),
            Point3(1, 0, 1),
        ),
        Tri(
            Point3(0.25, 0.25, 0),
            Point3(0, 0.25, 1),
            Point3(-0.65, 0.25, 1),
        ),
    ]
)

eye = Point3(0, 3, 0)
focus = Vec3(0, 0, 0)
up = Vec3(0, 0, 1)

fovy = 50.0
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
