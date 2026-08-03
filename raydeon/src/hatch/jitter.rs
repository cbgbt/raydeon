//! Position-hashed noise: the only source of randomness in the crate.
//!
//! Hatching needs a scattered, organic look, but a render must be
//! reproducible stroke for stroke. Hashing a sample's position instead of
//! drawing from a stateful generator gives both: the same point always
//! receives the same value, whatever order the samples are visited in and
//! however many threads visit them.

use crate::{CameraSpace, Point2, WPoint3};

/// A value in `[0, 1)` derived from `seed`.
pub(crate) fn jitter01(seed: u64) -> f64 {
    // splitmix64 finalizer: uniform enough for stochastic hatching.
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Seed for a world-space sample, quantized to roughly 5mm so that samples
/// which land on nearly the same spot jitter alike.
pub(crate) fn world_seed(point: WPoint3, salt: u64) -> u64 {
    mix(
        quantize(point.x, 200.0),
        quantize(point.y, 200.0),
        quantize(point.z, 200.0),
        salt,
    )
}

/// Seed for a screen-space sample, quantized to an eighth of a pixel.
pub(crate) fn screen_seed(point: Point2<CameraSpace>, salt: u64) -> u64 {
    mix(quantize(point.x, 8.0), quantize(point.y, 8.0), 0, salt)
}

fn quantize(value: f64, per_unit: f64) -> u64 {
    (value * per_unit).round() as i64 as u64
}

fn mix(x: u64, y: u64, z: u64, salt: u64) -> u64 {
    x.wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .wrapping_add(y.wrapping_mul(0x85eb_ca6b))
        .wrapping_add(z.wrapping_mul(0xc2b2_ae35))
        .wrapping_add(salt.wrapping_mul(0xd6e8_feb8_6659_fd93))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jitter_stays_in_the_unit_interval() {
        for seed in 0..10_000u64 {
            let value = jitter01(seed);
            assert!((0.0..1.0).contains(&value), "{value} out of range");
        }
    }

    #[test]
    fn the_same_point_always_jitters_the_same() {
        let point = WPoint3::new(1.25, -3.5, 0.75);
        assert_eq!(world_seed(point, 7), world_seed(point, 7));
        assert_eq!(
            jitter01(world_seed(point, 7)),
            jitter01(world_seed(point, 7))
        );
    }

    #[test]
    fn the_seed_changes_the_noise() {
        let point = WPoint3::new(1.25, -3.5, 0.75);
        assert_ne!(world_seed(point, 0), world_seed(point, 1));
    }

    #[test]
    fn nearby_points_within_the_quantum_share_a_seed() {
        let point = WPoint3::new(1.0, 2.0, 3.0);
        let nudged = WPoint3::new(1.001, 2.001, 3.001);
        assert_eq!(world_seed(point, 0), world_seed(nudged, 0));
    }

    #[test]
    fn distinct_points_jitter_differently() {
        let a = world_seed(WPoint3::new(1.0, 2.0, 3.0), 0);
        let b = world_seed(WPoint3::new(1.5, 2.0, 3.0), 0);
        assert_ne!(jitter01(a), jitter01(b));
    }

    #[test]
    fn screen_samples_hash_by_subpixel() {
        let point = Point2::<CameraSpace>::new(10.0, 20.0);
        assert_eq!(screen_seed(point, 3), screen_seed(point, 3));
        assert_ne!(
            screen_seed(point, 3),
            screen_seed(Point2::new(10.5, 20.0), 3)
        );
    }
}
