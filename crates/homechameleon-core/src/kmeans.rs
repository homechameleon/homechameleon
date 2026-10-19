//! Deterministic OKLab k-means. No random source: the same pixels always
//! produce the same centers, on native and on wasm.

use crate::color::Oklab;

#[derive(Clone, Copy, Debug)]
pub struct Cluster {
    pub center: Oklab,
    pub count: u32,
}

/// Lloyd's algorithm with farthest-point seeding.
/// Ties break toward the lower index, so the result does not depend on a hash seed.
pub fn cluster(points: &[Oklab], k: usize, iterations: usize) -> Vec<Cluster> {
    if points.is_empty() || k == 0 {
        return Vec::new();
    }
    let k = k.min(points.len());
    let mut centers = seed(points, k);
    let mut counts = vec![0u32; k];
    for _ in 0..iterations {
        let mut sum_l = vec![0.0; k];
        let mut sum_a = vec![0.0; k];
        let mut sum_b = vec![0.0; k];
        counts.fill(0);
        for point in points {
            let idx = nearest(point, &centers);
            sum_l[idx] += point.l;
            sum_a[idx] += point.a;
            sum_b[idx] += point.b;
            counts[idx] = counts[idx].saturating_add(1);
        }
        for i in 0..k {
            if counts[i] == 0 {
                continue;
            }
            let n = f64::from(counts[i]);
            centers[i] = Oklab {
                l: sum_l[i] / n,
                a: sum_a[i] / n,
                b: sum_b[i] / n,
            };
        }
    }
    // One last assignment so the reported counts match the final centers.
    counts.fill(0);
    for point in points {
        let idx = nearest(point, &centers);
        counts[idx] = counts[idx].saturating_add(1);
    }
    centers
        .into_iter()
        .zip(counts)
        .map(|(center, count)| Cluster { center, count })
        .collect()
}

fn seed(points: &[Oklab], k: usize) -> Vec<Oklab> {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&i, &j| {
        cmp_f64(points[i].l, points[j].l)
            .then(cmp_f64(points[i].a, points[j].a))
            .then(cmp_f64(points[i].b, points[j].b))
            .then(i.cmp(&j))
    });
    let mut centers = vec![points[order[order.len() / 2]]];
    while centers.len() < k {
        let mut best_i = 0usize;
        let mut best_d = -1.0;
        for (i, point) in points.iter().enumerate() {
            let d = centers
                .iter()
                .map(|c| point.distance(*c))
                .fold(f64::MAX, f64::min);
            if d > best_d + 1e-12 || ((d - best_d).abs() <= 1e-12 && i < best_i) {
                best_d = d;
                best_i = i;
            }
        }
        centers.push(points[best_i]);
    }
    centers
}

fn nearest(point: &Oklab, centers: &[Oklab]) -> usize {
    let mut best = 0usize;
    let mut best_d = f64::MAX;
    for (i, center) in centers.iter().enumerate() {
        let d = point.distance(*center);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best
}

fn cmp_f64(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab(l: f64, a: f64, b: f64) -> Oklab {
        Oklab { l, a, b }
    }

    #[test]
    fn two_groups_keep_their_counts() {
        let mut points = Vec::new();
        for _ in 0..10 {
            points.push(lab(0.2, 0.0, 0.0));
        }
        for _ in 0..30 {
            points.push(lab(0.8, 0.1, -0.05));
        }
        let once = cluster(&points, 2, 8);
        let twice = cluster(&points, 2, 8);
        assert_eq!(once.len(), 2);
        let mut counts: Vec<u32> = once.iter().map(|c| c.count).collect();
        counts.sort_unstable();
        assert_eq!(counts, vec![10, 30]);
        for (a, b) in once.iter().zip(&twice) {
            assert_eq!(a.count, b.count);
            assert!((a.center.l - b.center.l).abs() < 1e-12);
        }
    }
}
