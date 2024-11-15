use euclid::approxeq::ApproxEq;
use euclid::*;
use rayon::prelude::*;

pub type LineSegment<Space> = (Point3D<f64, Space>, Point3D<f64, Space>);

pub fn transform_segment<T, Dst>(
    path: LineSegment<T>,
    transformation: &Transform3D<f64, T, Dst>,
) -> Option<LineSegment<Dst>>
where
    T: Send + Sync + Sized,
    Dst: Send + Sync + Sized,
{
    let (p1, p2) = path;
    let p1t = transformation.transform_point3d(p1);
    let p2t = transformation.transform_point3d(p2);
    p1t.and_then(|p1| p2t.map(|p2| (p1, p2)))
}

pub fn simplify_segments<T>(paths: &[LineSegment<T>], threshold: f64) -> Vec<LineSegment<T>> {
    let eps: Point3D<f64, T> = Point3D::new(threshold, threshold, threshold);
    let mut npaths = Vec::new();
    let mut curr_line: Option<(Point3D<f64, T>, Point3D<f64, T>)> = None;
    let mut curr_pushed = true;
    for (v1, v2) in paths {
        let v1 = *v1;
        let v2 = *v2;
        if curr_line.is_none() {
            curr_line = Some((v1, v2));
            curr_pushed = false;
        } else {
            let (cv1, cv2) = curr_line.unwrap();
            let curr_line_dir = (cv2 - cv1).normalize();
            let nline_dir = (v2 - v1).normalize();

            let same_dir = curr_line_dir.approx_eq_eps(&nline_dir, &eps.to_vector())
                || curr_line_dir.approx_eq_eps(&-nline_dir, &eps.to_vector());

            if same_dir {
                if cv1.approx_eq_eps(&v1, &eps) {
                    curr_line = Some((v2, cv2));
                    curr_pushed = false;
                } else if cv1.approx_eq_eps(&v2, &eps) {
                    curr_line = Some((v1, cv2));
                    curr_pushed = false;
                } else if cv2.approx_eq_eps(&v1, &eps) {
                    curr_line = Some((v2, cv1));
                    curr_pushed = false;
                } else if cv2.approx_eq_eps(&v2, &eps) {
                    curr_line = Some((v1, cv1));
                    curr_pushed = false;
                } else {
                    npaths.push((cv1, cv2));
                    curr_line = Some((v1, v2));
                    curr_pushed = true;
                }
            } else {
                npaths.push((cv1, cv2));
                curr_line = Some((v1, v2));
                curr_pushed = false;
            }
        }
    }

    if curr_line.is_some() && !curr_pushed {
        npaths.push(curr_line.unwrap());
    }

    npaths
}
