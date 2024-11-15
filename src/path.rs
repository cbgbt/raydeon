use euclid::approxeq::ApproxEq;
use euclid::*;

#[derive(Debug, Clone)]
pub struct Paths<T> {
    pub lines: Vec<(Point3D<f64, T>, Point3D<f64, T>)>,
}

impl<T> Paths<T> {
    pub fn new(lines: Vec<(Point3D<f64, T>, Point3D<f64, T>)>) -> Paths<T> {
        Paths { lines }
    }

    pub fn join(&mut self, mut other: Paths<T>) {
        self.lines.append(&mut other.lines);
    }

    pub fn transform<Dst>(&self, transformation: Transform3D<f64, T, Dst>) -> Paths<Dst> {
        let mut new_lines = Vec::new();
        for (p1, p2) in &self.lines {
            let p1t = transformation.transform_point3d(*p1);
            let p2t = transformation.transform_point3d(*p2);

            if p1t.is_none() || p2t.is_none() {
                continue;
            }
            new_lines.push((p1t.unwrap(), p2t.unwrap()));
        }
        Paths::new(new_lines)
    }

    pub fn chop(&self, step: f64) -> Paths<T> {
        let mut nlines = Vec::new();
        for (p1, p2) in &self.lines {
            let (p1, p2) = (*p1, *p2);
            let v = p2 - p1;
            let l = v.length();
            let stepv = v.normalize() * step;

            let mut curr = p1;
            let mut d = step;
            while d < l {
                let next = curr + stepv;
                nlines.push((curr, next));
                curr = next;
                d += step;
            }
            nlines.push((curr, p2))
        }

        Paths::new(nlines)
    }

    pub fn simplify(mut self, threshold: f64) -> Paths<T> {
        let eps: Point3D<f64, T> = Point3D::new(threshold, threshold, threshold);
        let mut npaths = Vec::new();
        let mut curr_line: Option<(Point3D<f64, T>, Point3D<f64, T>)> = None;
        let mut curr_pushed = true;
        for (v1, v2) in self.lines.drain(..) {
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

        Paths::new(npaths)
    }
}
