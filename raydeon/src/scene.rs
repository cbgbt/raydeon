use bvh::BVHTree;
use camera::{Observation, Perspective};
use path::{LineSegment2D, SlicedSegment3D};
use rayon::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::*;

pub struct SceneCamera<'s, P>
where
    P: PathMeta,
{
    camera: Camera<Perspective, Observation>,
    scene: &'s Scene<P>,
}

impl<'a, P> SceneCamera<'a, P>
where
    P: PathMeta,
{
    fn clip_filter<M: PathMeta>(&self, path: &LineSegment3D<WorldSpace, M>) -> bool {
        self.scene
            .visible(self.camera.observation.eye, path.midpoint())
    }

    pub fn render(&self) -> Vec<LineSegment2D<CameraSpace>> {
        let parent_paths: Vec<LineSegment3D<WorldSpace, P>> = self
            .scene
            .geometry
            .par_iter()
            .flat_map(|s| s.paths(&self.camera))
            .collect();

        info!(
            "Caching line segment chunks based on camera position, starting with {} segments",
            parent_paths.len()
        );

        let mut paths: Vec<SlicedSegment3D<WorldSpace, P>> = parent_paths
            .iter()
            .filter_map(|path| self.camera.chop_segment(path))
            .collect();

        let path_count: usize = paths
            .par_iter()
            .map(|subsegments| subsegments.num_subsegments())
            .sum();

        info!(
            "Done caching line segment chunks, created {} segment chunks",
            path_count
        );

        info!("Clipping occluded and distant segment chunks");

        let transformation: Transform3<WorldSpace, CameraSpace> =
            self.camera.camera_transformation();

        let paths: Vec<_> = paths
            .par_iter_mut()
            .flat_map(|path_group| {
                let to_remove: Vec<usize> = path_group
                    .subsegments()
                    .enumerate()
                    .par_bridge()
                    .filter_map(|(ndx, path)| {
                        let from_cam = path.midpoint() - self.camera.observation.eye;
                        let close_enough = from_cam.length() < self.camera.perspective.zfar;
                        let visible = close_enough && self.clip_filter(&path);
                        (!visible).then_some(ndx)
                    })
                    .collect();
                tracing::debug!("Removed {:?} subsegments", to_remove);
                to_remove
                    .into_iter()
                    .for_each(|ndx| path_group.remove_subsegment(ndx));
                path_group.join_slices()
            })
            .filter_map(|path| path.transform(&transformation))
            .map(LineSegment3D::xy)
            .collect();

        info!("{} paths remain after clipping", paths.len());

        paths
    }
}

#[derive(Debug)]
pub struct Scene<P>
where
    P: PathMeta,
{
    geometry: Vec<Arc<dyn Shape<WorldSpace, P>>>,
    bvh: BVHTree<WorldSpace>,
}

impl<P> Scene<P>
where
    P: PathMeta,
{
    pub fn new(geometry: Vec<Arc<dyn Shape<WorldSpace, P>>>) -> Scene<P> {
        let collision_geometry: Vec<_> = geometry
            .iter()
            .filter_map(|s| s.collision_geometry())
            .flatten()
            .collect();
        let bvh = BVHTree::new(&collision_geometry);
        Scene { geometry, bvh }
    }

    pub fn attach_camera(&self, camera: Camera<Perspective, Observation>) -> SceneCamera<P> {
        SceneCamera {
            camera,
            scene: self,
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
