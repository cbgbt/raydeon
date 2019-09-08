use euclid::*;

use crate::CPoint3;

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
            let p1t = transformation.transform_point3d(*p1).unwrap();
            let p2t = transformation.transform_point3d(*p2).unwrap();
            new_lines.push((p1t, p2t));
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
                nlines.push((
                    curr,
                    next
                ));
                curr = next;
                d += step;
            }
            nlines.push((curr, p2))
        }

        dbg!(&nlines);
        Paths::new(nlines)
    }

    pub fn simplify(mut self, threshold: f64) -> Paths<T> {
        self
    }
}