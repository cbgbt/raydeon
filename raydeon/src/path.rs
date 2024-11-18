use euclid::approxeq::ApproxEq;
use euclid::*;

#[derive(Debug, Copy, Clone)]
pub struct LineSegment3D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub p1: Point3D<f64, Space>,
    pub p2: Point3D<f64, Space>,
    pub tag: usize,
}

impl<Space> LineSegment3D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new(p1: Point3D<f64, Space>, p2: Point3D<f64, Space>) -> Self {
        Self::tagged(p1, p2, 0)
    }

    pub fn cast_unit<U>(self) -> LineSegment3D<U>
    where
        U: Copy + Clone + std::fmt::Debug,
    {
        LineSegment3D::new(self.p1.cast_unit(), self.p2.cast_unit())
    }

    pub fn xy(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.xy(), self.p2.xy())
    }

    pub fn yz(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.yz(), self.p2.yz())
    }

    pub fn xz(self) -> LineSegment2D<Space> {
        LineSegment2D::new(self.p1.xz(), self.p2.xz())
    }

    pub fn tagged(p1: Point3D<f64, Space>, p2: Point3D<f64, Space>, tag: usize) -> Self {
        Self { p1, p2, tag }
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform3D<f64, Space, Dst>,
    ) -> Option<LineSegment3D<Dst>>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        let p1t = transformation.transform_point3d(p1);
        let p2t = transformation.transform_point3d(p2);
        p1t.and_then(|p1| p2t.map(|p2| LineSegment3D::tagged(p1, p2, self.tag)))
    }
}

pub fn simplify_3d_segments<T>(paths: &[LineSegment3D<T>], threshold: f64) -> Vec<LineSegment3D<T>>
where
    T: Copy + Clone + std::fmt::Debug,
{
    let eps: Point3D<f64, T> = Point3D::new(threshold, threshold, threshold);
    let mut npaths = Vec::new();
    let mut curr_line: Option<LineSegment3D<T>> = None;
    let mut curr_pushed = true;
    for path in paths {
        let v1 = path.p1;
        let v2 = path.p2;
        if curr_line.is_none() {
            curr_line = Some(*path);
            curr_pushed = false;
        } else {
            let (cv1, cv2) = curr_line
                .as_ref()
                .map(|curr_line| (curr_line.p1, curr_line.p2))
                .unwrap();
            let curr_line_dir = (cv2 - cv1).normalize();
            let nline_dir = (v2 - v1).normalize();

            let same_dir = curr_line_dir.approx_eq_eps(&nline_dir, &eps.to_vector())
                || curr_line_dir.approx_eq_eps(&-nline_dir, &eps.to_vector());

            let same_tag = curr_line.as_ref().unwrap().tag == path.tag;

            if same_tag && same_dir {
                if cv1.approx_eq_eps(&v1, &eps) {
                    curr_line = Some(LineSegment3D::tagged(v2, cv2, path.tag));
                    curr_pushed = false;
                } else if cv1.approx_eq_eps(&v2, &eps) {
                    curr_line = Some(LineSegment3D::tagged(v1, cv2, path.tag));
                    curr_pushed = false;
                } else if cv2.approx_eq_eps(&v1, &eps) {
                    curr_line = Some(LineSegment3D::tagged(v2, cv1, path.tag));
                    curr_pushed = false;
                } else if cv2.approx_eq_eps(&v2, &eps) {
                    curr_line = Some(LineSegment3D::tagged(v1, cv1, path.tag));
                    curr_pushed = false;
                } else {
                    npaths.push(curr_line.unwrap());
                    curr_line = Some(LineSegment3D::tagged(v1, v2, path.tag));
                    curr_pushed = true;
                }
            } else {
                npaths.push(curr_line.unwrap());
                curr_line = Some(*path);
                curr_pushed = false;
            }
        }
    }

    if let Some(curr_line) = curr_line {
        if !curr_pushed {
            npaths.push(curr_line);
        }
    }

    npaths
}

#[derive(Debug, Copy, Clone)]
pub struct LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub p1: Point2D<f64, Space>,
    pub p2: Point2D<f64, Space>,
    pub tag: usize,
}

impl<Space> LineSegment2D<Space>
where
    Space: Copy + Clone + std::fmt::Debug,
{
    pub fn new(p1: Point2D<f64, Space>, p2: Point2D<f64, Space>) -> Self {
        Self::tagged(p1, p2, 0)
    }

    pub fn tagged(p1: Point2D<f64, Space>, p2: Point2D<f64, Space>, tag: usize) -> Self {
        Self { p1, p2, tag }
    }

    pub fn cast_unit<U>(self) -> LineSegment2D<U>
    where
        U: Copy + Clone + std::fmt::Debug,
    {
        LineSegment2D::new(self.p1.cast_unit(), self.p2.cast_unit())
    }

    pub fn transform<Dst>(
        &self,
        transformation: &Transform2D<f64, Space, Dst>,
    ) -> LineSegment2D<Dst>
    where
        Dst: Copy + Clone + std::fmt::Debug,
    {
        let (p1, p2) = (self.p1, self.p2);
        LineSegment2D::tagged(
            transformation.transform_point(p1),
            transformation.transform_point(p2),
            self.tag,
        )
    }
}
