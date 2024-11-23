use bvh::BVHTree;
use camera::{Observation, Perspective};
use path::{simplify_3d_segments, LineSegment2D};
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

pub struct SceneCamera<'s> {
    camera: Camera<Perspective, Observation>,
    scene: &'s Scene,
    paths: Vec<Vec<LineSegment3D<WorldSpace>>>,
    path_count: usize,
}

impl<'a> SceneCamera<'a> {
    fn clip_filter(&self, path: &LineSegment3D<WorldSpace>) -> bool {
        let (p1, p2) = (path.p1, path.p2);
        let midpoint = p1 + ((p2 - p1) / 2.0);
        self.scene.visible(self.camera.observation.eye, midpoint)
    }

    pub fn render(&self) -> Vec<LineSegment2D<CameraSpace>> {
        info!(
            "Clipping occluded and distant segment chunks, started with {} segments",
            self.path_count
        );

        let transformation: Transform3<WorldSpace, CameraSpace> =
            self.camera.camera_transformation();

        let paths: Vec<_> = self
            .paths
            .par_iter()
            .map(|path_group| {
                path_group
                    .par_iter()
                    .filter(|path| {
                        let close_enough = (path.p1.to_vector()
                            - self.camera.observation.eye.to_vector())
                        .length()
                            < self.camera.perspective.zfar;
                        close_enough && self.clip_filter(path)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .flat_map(|path_group| simplify_3d_segments(&path_group, 1.0e-6))
            .filter_map(|path| path.transform(&transformation))
            .map(LineSegment3D::xy)
            .collect();

        info!("{} paths remain after clipping", paths.len());

        paths
    }
}

#[derive(Debug)]
pub struct Scene {
    geometry: Vec<Arc<dyn Shape<WorldSpace>>>,
    bvh: BVHTree<WorldSpace>,
}

impl Scene {
    pub fn new(geometry: Vec<Arc<dyn Shape<WorldSpace>>>) -> Scene {
        let collision_geometry: Vec<_> = geometry
            .iter()
            .filter_map(|s| s.collision_geometry())
            .flatten()
            .collect();
        let bvh = BVHTree::new(&collision_geometry);
        Scene { geometry, bvh }
    }

    pub fn attach_camera(&self, camera: Camera<Perspective, Observation>) -> SceneCamera {
        info!("Caching line segment chunks based on new camera attachment");
        let paths: Vec<Vec<LineSegment3D<WorldSpace>>> = self
            .geometry
            .par_iter()
            .map(|s| s.paths(&camera))
            .flat_map(|paths| {
                paths
                    .par_iter()
                    .map(|path| camera.chop_segment(path))
                    .collect::<Vec<_>>()
            })
            .collect();
        let path_count = paths.par_iter().map(|path_group| path_group.len()).sum();
        info!(
            "Done caching line segment chunks, created {} segment chunks",
            path_count
        );

        SceneCamera {
            camera,
            scene: self,
            paths,
            path_count,
        }
    }

    /// Find's the closest intersection point to geometry in the scene, if any
    fn intersects(&self, ray: Ray) -> Option<HitData> {
        self.bvh.intersects(ray)
    }

    /// Returns whether or not the given camera has a clear line of sight to a given point.
    fn visible(&self, from: WPoint3, point: WPoint3) -> bool {
        let v = from - point;
        let r = Ray::new(point, v.normalize());

        match self.intersects(r) {
            Some(hitdata) => {
                let diff = (hitdata.dist_to - v.length()).abs();
                diff < 1.0e-1
            }
            None => true,
        }
    }
}
