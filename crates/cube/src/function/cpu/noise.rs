//! Noise functions for CPU evaluation
//!
//! Implements deterministic 3D noise functions that can be used in expressions.
//! These implementations match the GPU (WGSL) versions for consistency.

/// Permutation table for gradient noise (deterministic, based on seed)
#[allow(dead_code)]
fn perm(seed: u32) -> [u8; 256] {
    let mut table = [0u8; 256];
    for (i, item) in table.iter_mut().enumerate() {
        *item = i as u8;
    }

    // Fisher-Yates shuffle with deterministic random
    let mut state = seed.wrapping_mul(0x9E3779B9);
    for i in (1..256).rev() {
        state = state.wrapping_mul(0x5BD1E995);
        state ^= state >> 15;
        let j = (state as usize) % (i + 1);
        table.swap(i, j);
    }

    table
}

/// Hash function for gradient lookup
fn grad_hash(x: i32, y: i32, z: i32, seed: u32) -> f64 {
    let h = x
        .wrapping_add(y.wrapping_mul(57))
        .wrapping_add(z.wrapping_mul(113))
        .wrapping_add(seed as i32);
    let h = (h as u32).wrapping_mul(0x27d4eb2d);
    let h = h ^ (h >> 15);
    (h & 0x7FFFFFFF) as f64 / 0x7FFFFFFF as f64
}

/// Gradient vectors for 3D noise
#[allow(dead_code)]
const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// Dot product of gradient and distance vector
#[allow(dead_code)]
fn grad_dot(hash: i32, x: f64, y: f64, z: f64) -> f64 {
    let g = &GRADIENTS[(hash as usize) % 12];
    g[0] * x + g[1] * y + g[2] * z
}

/// Fade function for smooth interpolation: 6t^5 - 15t^4 + 10t^3
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation
#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Simple value noise 3D (fast, deterministic)
///
/// Returns values in the range [-1, 1]
pub fn noise3(x: f64, y: f64, z: f64, seed: u32) -> f64 {
    // Integer coordinates
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;

    // Fractional coordinates
    let xf = x - xi as f64;
    let yf = y - yi as f64;
    let zf = z - zi as f64;

    // Fade curves for interpolation
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    // Hash coordinates of cube corners
    let h000 = grad_hash(xi, yi, zi, seed);
    let h001 = grad_hash(xi, yi, zi + 1, seed);
    let h010 = grad_hash(xi, yi + 1, zi, seed);
    let h011 = grad_hash(xi, yi + 1, zi + 1, seed);
    let h100 = grad_hash(xi + 1, yi, zi, seed);
    let h101 = grad_hash(xi + 1, yi, zi + 1, seed);
    let h110 = grad_hash(xi + 1, yi + 1, zi, seed);
    let h111 = grad_hash(xi + 1, yi + 1, zi + 1, seed);

    // Trilinear interpolation
    let x00 = lerp(h000, h100, u);
    let x01 = lerp(h001, h101, u);
    let x10 = lerp(h010, h110, u);
    let x11 = lerp(h011, h111, u);

    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);

    let result = lerp(y0, y1, w);

    // Map from [0, 1] to [-1, 1]
    result * 2.0 - 1.0
}

/// Perlin gradient noise 3D (higher quality, slightly slower)
///
/// Returns values in the range approximately [-1, 1]
#[allow(dead_code)]
pub fn perlin3(x: f64, y: f64, z: f64, seed: u32) -> f64 {
    let perm_table = perm(seed);

    // Integer coordinates
    let xi = x.floor() as i32 & 255;
    let yi = y.floor() as i32 & 255;
    let zi = z.floor() as i32 & 255;

    // Fractional coordinates
    let xf = x - x.floor();
    let yf = y - y.floor();
    let zf = z - z.floor();

    // Fade curves
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);

    // Hash cube corners
    let aa = perm_table[(perm_table[(perm_table[xi as usize] as i32 + yi) as usize & 255] as i32
        + zi) as usize
        & 255] as i32;
    let ab = perm_table[(perm_table[(perm_table[xi as usize] as i32 + yi + 1) as usize & 255]
        as i32
        + zi) as usize
        & 255] as i32;
    let ba = perm_table[(perm_table
        [(perm_table[(xi + 1) as usize & 255] as i32 + yi) as usize & 255]
        as i32
        + zi) as usize
        & 255] as i32;
    let bb = perm_table[(perm_table
        [(perm_table[(xi + 1) as usize & 255] as i32 + yi + 1) as usize & 255]
        as i32
        + zi) as usize
        & 255] as i32;

    // Gradient dot products
    let g000 = grad_dot(aa, xf, yf, zf);
    let g001 = grad_dot(aa + 1, xf, yf, zf - 1.0);
    let g010 = grad_dot(ab, xf, yf - 1.0, zf);
    let g011 = grad_dot(ab + 1, xf, yf - 1.0, zf - 1.0);
    let g100 = grad_dot(ba, xf - 1.0, yf, zf);
    let g101 = grad_dot(ba + 1, xf - 1.0, yf, zf - 1.0);
    let g110 = grad_dot(bb, xf - 1.0, yf - 1.0, zf);
    let g111 = grad_dot(bb + 1, xf - 1.0, yf - 1.0, zf - 1.0);

    // Trilinear interpolation
    let x00 = lerp(g000, g100, u);
    let x01 = lerp(g001, g101, u);
    let x10 = lerp(g010, g110, u);
    let x11 = lerp(g011, g111, u);

    let y0 = lerp(x00, x10, v);
    let y1 = lerp(x01, x11, v);

    lerp(y0, y1, w)
}

/// Fractal Brownian Motion (FBM) - layered noise
///
/// Combines multiple octaves of noise with decreasing amplitude and increasing frequency.
/// Returns values in approximately the range [-1, 1].
pub fn fbm(x: f64, y: f64, z: f64, octaves: u32, seed: u32) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves.min(8) {
        let octave_seed = seed.wrapping_add(i);
        value += amplitude * noise3(x * frequency, y * frequency, z * frequency, octave_seed);
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

/// Turbulence noise - absolute value of FBM layers
///
/// Creates sharper, more dramatic patterns by using absolute values.
/// Returns values in the range [0, 1].
pub fn turbulence(x: f64, y: f64, z: f64, octaves: u32, seed: u32) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for i in 0..octaves.min(8) {
        let octave_seed = seed.wrapping_add(i);
        value += amplitude * noise3(x * frequency, y * frequency, z * frequency, octave_seed).abs();
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value / max_value
}

/// Ridged noise - inverted absolute FBM for ridge-like patterns
///
/// Creates mountain ridge-like patterns.
/// Returns values in the range [0, 1].
#[allow(dead_code)]
pub fn ridged(x: f64, y: f64, z: f64, octaves: u32, seed: u32) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut weight = 1.0;

    for i in 0..octaves.min(8) {
        let octave_seed = seed.wrapping_add(i);
        let signal = 1.0 - noise3(x * frequency, y * frequency, z * frequency, octave_seed).abs();
        let signal = signal * signal * weight;
        weight = (signal * 2.0).clamp(0.0, 1.0);
        value += signal * amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise3_range() {
        for seed in [0, 42, 12345] {
            for i in 0..100 {
                let x = (i as f64) * 0.1;
                let y = (i as f64) * 0.07;
                let z = (i as f64) * 0.13;
                let v = noise3(x, y, z, seed);
                assert!(
                    v >= -1.0 && v <= 1.0,
                    "noise3({}, {}, {}, {}) = {} out of range",
                    x,
                    y,
                    z,
                    seed,
                    v
                );
            }
        }
    }

    #[test]
    fn test_noise3_deterministic() {
        let seed = 42;
        let v1 = noise3(1.5, 2.3, 3.7, seed);
        let v2 = noise3(1.5, 2.3, 3.7, seed);
        assert!((v1 - v2).abs() < 1e-10);
    }

    #[test]
    fn test_noise3_different_seeds() {
        let v1 = noise3(1.0, 2.0, 3.0, 0);
        let v2 = noise3(1.0, 2.0, 3.0, 1);
        // Different seeds should give different results (with high probability)
        assert!((v1 - v2).abs() > 0.001);
    }

    #[test]
    fn test_fbm_range() {
        let seed = 42;
        for i in 0..100 {
            let x = (i as f64) * 0.1;
            let y = (i as f64) * 0.07;
            let z = (i as f64) * 0.13;
            let v = fbm(x, y, z, 4, seed);
            assert!(
                v >= -1.0 && v <= 1.0,
                "fbm({}, {}, {}, 4, {}) = {} out of range",
                x,
                y,
                z,
                seed,
                v
            );
        }
    }

    #[test]
    fn test_turbulence_range() {
        let seed = 42;
        for i in 0..100 {
            let x = (i as f64) * 0.1;
            let y = (i as f64) * 0.07;
            let z = (i as f64) * 0.13;
            let v = turbulence(x, y, z, 4, seed);
            assert!(
                v >= 0.0 && v <= 1.0,
                "turbulence({}, {}, {}, 4, {}) = {} out of range",
                x,
                y,
                z,
                seed,
                v
            );
        }
    }

    #[test]
    fn test_noise_continuity() {
        // Test that noise varies smoothly (no sudden jumps)
        let seed = 42;
        let mut prev = noise3(0.0, 0.0, 0.0, seed);
        for i in 1..100 {
            let t = (i as f64) * 0.01;
            let curr = noise3(t, 0.0, 0.0, seed);
            let diff = (curr - prev).abs();
            assert!(
                diff < 0.5,
                "Noise jump too large at t={}: {} -> {} (diff={})",
                t,
                prev,
                curr,
                diff
            );
            prev = curr;
        }
    }
}
