use crate::*;

pub struct LookingCamera {
    eye: WPoint3,
    center: WVec3,
    up: WVec3,
    matrix: WCTransform,
}

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    eye: WPoint3,
    center: WVec3,
    up: WVec3,

    fovy: f64,
    width: f64,
    height: f64,
    aspect: f64,
    znear: f64,
    zfar: f64,

    matrix: Transform3D<f64, WorldSpace, CanvasSpace>,
}

impl Camera {
    // TODO testme
    pub fn look_at(eye: WPoint3, center: WVec3, up: WVec3) -> LookingCamera {
        let up = up.normalize();
        let f = (center - eye.to_vector()).normalize();
        let s = f.cross(up).normalize();
        let u = s.cross(f).normalize();

        let look_at_matrix = CWTransform::column_major(
            s.x, u.x, -f.x, eye.x,
            s.y, u.y, -f.y, eye.y,
            s.z, u.z, -f.z, eye.z,
            0.0, 0.0, 0.0, 1.0,
        )
        .inverse()
        .unwrap();

        let matrix = look_at_matrix;
        LookingCamera {
            eye,
            center,
            up,
            matrix
        }
    }
}

fn frustum(l: f64, r: f64, b: f64, t: f64, n: f64, f: f64) -> Transform3D<f64, CameraSpace, CanvasSpace> {
    let t1 = 2.0 * n;
    let t2 = r - l;
    let t3 = t - b;
    let t4 = f - n;

    Transform3D::column_major(
        t1 / t2, 0.0, (r + l) / t2, 0.0,
        0.0, t1 / t3, (t + b) / t3, 0.0,
        0.0, 0.0, (-f - n) / t4, (-t1 * f) / t4,
        0.0, 0.0, -1.0, 0.0,
    )
}

impl LookingCamera {
    pub fn perspective(self, fovy: f64, width: f64, height: f64, znear: f64, zfar: f64) -> Camera {
        let aspect = width / height;
        let ymax = znear * (fovy * std::f64::consts::PI / 360.0).tan();
        let xmax = ymax * aspect;

        let my_frustum = frustum(-xmax, xmax, -ymax, ymax, znear, zfar);
        let matrix = my_frustum.pre_transform(&self.matrix);

        Camera {
            eye: self.eye,
            center: self.center,
            up: self.up,
            fovy,
            width,
            height,
            aspect,
            znear,
            zfar,
            matrix,
        }
    }
}

pub struct Scene {
    geometry: Vec<Box<dyn Shape>>,
}

impl Scene {
    pub fn new(geometry: Vec<Box<dyn Shape>>) -> Scene {
        Scene { geometry }
    }

    fn intersect(&self, ray: Ray) -> Option<HitData> {
        let mut hit = None;
        let mut mindist = std::f64::MAX;
        for geometry in &self.geometry {
            match geometry.hit_by(&ray) {
                Some(hitdata) => {
                    if hitdata.dist_to < mindist {
                        mindist = hitdata.dist_to;
                        hit = Some(hitdata);
                    }
                },
                None => (),
            }
        }
        hit
    }

    fn visible(&self, camera: &Camera, point: WPoint3) -> bool {
        let v = camera.eye - point;
        let r = Ray::new(point, v.normalize());

        match self.intersect(r) {
            Some(hitdata) => {
                hitdata.dist_to >= v.length()
            },
            None => true
        }
    }

    fn clip_filter(&self, camera: &Camera, paths: Paths<WorldSpace>) -> Paths<WorldSpace> {
        let mut npaths = Vec::new();
        for (p1, p2) in &paths.lines {
            let (p1, p2) = (*p1, *p2);
            let midpoint = p1 + ((p2 - p1) / 2.0);

            if self.visible(camera, midpoint) {
                npaths.push((p1, p2));
            }
        }
        
        Paths::new(npaths)
    }

    pub fn render(&self, camera: Camera, step_size: f64) -> Paths<CameraSpace> {
        let mut paths = Paths::new(vec![]);
        for shape in &self.geometry {
            paths.join(shape.paths());
        }

        if step_size > 0.0 {
            paths = paths.chop(step_size);
        }

        let paths = self.clip_filter(&camera, paths);
        let mut paths = paths.transform(camera.matrix);

        if step_size > 0.0 {
            paths = paths.simplify(1.0e-6);
        }

        let translation = Transform3D::create_translation(1.0, 1.0, 0.0)
            .post_scale(camera.width / 2.0, camera.height / 2.0, 0.0);

        paths.transform(translation)
    }
}
