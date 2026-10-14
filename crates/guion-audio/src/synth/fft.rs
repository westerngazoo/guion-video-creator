//! A real FFT in plain `std` Rust, for zero-phase filters and convolution
//! (R-0008 AC5, AC6, AC11).
//!
//! Radix-2, iterative, single-threaded, in a fixed order of operations, so the
//! same input gives the same bits on the same platform. Every twiddle is
//! computed directly with `sin`/`cos` of its own index (no recurrence), which
//! keeps the error at a few ulps for the 2²³-point transforms a 90 s reel
//! needs. A real transform of `n` points runs as a complex one of `n / 2`.

use std::f64::consts::TAU;

/// A complex number, just enough for the transform.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }
    fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
    fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
    fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
    fn conj(self) -> Complex {
        Complex::new(self.re, -self.im)
    }
    fn scale(self, k: f64) -> Complex {
        Complex::new(self.re * k, self.im * k)
    }
}

/// The smallest power of two that is at least `n` (and at least 2).
pub fn next_pow2(n: usize) -> usize {
    n.max(2).next_power_of_two()
}

/// `W^k = e^{-2πik/n}` for `k < n/2`: the twiddles of an `n`-point real
/// transform, which also serve its `n/2`-point complex half (stride 2).
fn twiddles(n: usize) -> Vec<Complex> {
    (0..n / 2)
        .map(|k| {
            let a = -TAU * k as f64 / n as f64;
            Complex::new(a.cos(), a.sin())
        })
        .collect()
}

/// In-place complex FFT of `a` (length `m`, a power of two), with the
/// twiddles of a transform of `2m` points. `inverse` conjugates the twiddles
/// and does not scale.
fn complex_fft(a: &mut [Complex], tw: &[Complex], inverse: bool) {
    let m = a.len();
    let n = 2 * m;
    let mut j = 0usize;
    for i in 1..m {
        let mut bit = m >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= m {
        let half = len / 2;
        let stride = n / len;
        for start in (0..m).step_by(len) {
            for k in 0..half {
                let w = tw[k * stride];
                let w = if inverse { w.conj() } else { w };
                let u = a[start + k];
                let v = a[start + k + half].mul(w);
                a[start + k] = u.add(v);
                a[start + k + half] = u.sub(v);
            }
        }
        len <<= 1;
    }
}

/// The spectrum of `x` zero-padded to `n` points (`n` a power of two ≥ 2):
/// bins `0..=n/2`.
pub fn rfft(x: &[f64], n: usize) -> Vec<Complex> {
    debug_assert!(n.is_power_of_two() && n >= 2);
    let m = n / 2;
    let tw = twiddles(n);
    let at = |i: usize| x.get(i).copied().unwrap_or(0.0);
    let mut z: Vec<Complex> = (0..m)
        .map(|k| Complex::new(at(2 * k), at(2 * k + 1)))
        .collect();
    complex_fft(&mut z, &tw, false);
    let mut out = Vec::with_capacity(m + 1);
    for k in 0..=m {
        let zk = z[k % m];
        let zc = z[(m - k) % m].conj();
        let even = zk.add(zc).scale(0.5);
        // (zk − zc) / 2i
        let d = zk.sub(zc).scale(0.5);
        let odd = Complex::new(d.im, -d.re);
        let w = if k < m {
            tw[k]
        } else {
            Complex::new(-1.0, 0.0)
        };
        out.push(even.add(w.mul(odd)));
    }
    out
}

/// The inverse of [`rfft`]: `n` real samples from bins `0..=n/2`.
pub fn irfft(spec: &[Complex], n: usize) -> Vec<f64> {
    debug_assert!(n.is_power_of_two() && n >= 2 && spec.len() == n / 2 + 1);
    let m = n / 2;
    let tw = twiddles(n);
    let mut z: Vec<Complex> = (0..m)
        .map(|k| {
            let xk = spec[k];
            let xc = spec[m - k].conj();
            let even = xk.add(xc).scale(0.5);
            let odd = xk.sub(xc).scale(0.5).mul(tw[k].conj());
            // even + i·odd
            Complex::new(even.re - odd.im, even.im + odd.re)
        })
        .collect();
    complex_fft(&mut z, &tw, true);
    let k = 1.0 / m as f64;
    let mut out = Vec::with_capacity(n);
    for c in z {
        out.push(c.re * k);
        out.push(c.im * k);
    }
    out
}

/// Multiplies the spectrum of `x` by a real, zero-phase `gain(f)` (f in Hz)
/// and returns `x.len()` samples. `x` is zero-padded to the next power of two
/// above its length, so the filter does not wrap around.
pub fn fft_filter(x: &[f64], rate: f64, gain: impl Fn(f64) -> f64) -> Vec<f64> {
    if x.is_empty() {
        return Vec::new();
    }
    let n = next_pow2(x.len() + 1);
    let mut spec = rfft(x, n);
    for (k, c) in spec.iter_mut().enumerate() {
        *c = c.scale(gain(k as f64 * rate / n as f64));
    }
    let mut y = irfft(&spec, n);
    y.truncate(x.len());
    y
}

/// Linear convolution of `x` with `h`, truncated to `x.len()` (the tail past
/// the end of `x` is dropped, as a reel that ends ends).
pub fn convolve(x: &[f64], h: &[f64]) -> Vec<f64> {
    if x.is_empty() || h.is_empty() {
        return vec![0.0; x.len()];
    }
    let n = next_pow2(x.len() + h.len());
    let a = rfft(x, n);
    let b = rfft(h, n);
    let prod: Vec<Complex> = a.iter().zip(&b).map(|(p, q)| p.mul(*q)).collect();
    let mut y = irfft(&prod, n);
    y.truncate(x.len());
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dft(x: &[f64], n: usize) -> Vec<Complex> {
        (0..=n / 2)
            .map(|k| {
                let mut s = Complex::default();
                for (i, v) in x.iter().enumerate() {
                    let a = -TAU * (k * i) as f64 / n as f64;
                    s = s.add(Complex::new(v * a.cos(), v * a.sin()));
                }
                s
            })
            .collect()
    }

    #[test]
    fn rfft_matches_the_definition_and_inverts() {
        let x: Vec<f64> = (0..37)
            .map(|i| ((i * 7919) % 101) as f64 / 50.0 - 1.0)
            .collect();
        let n = 64;
        let a = rfft(&x, n);
        let b = dft(&x, n);
        for (p, q) in a.iter().zip(&b) {
            assert!((p.re - q.re).abs() < 1e-9 && (p.im - q.im).abs() < 1e-9);
        }
        let y = irfft(&a, n);
        for (i, v) in y.iter().enumerate() {
            let want = x.get(i).copied().unwrap_or(0.0);
            assert!((v - want).abs() < 1e-12, "{i}: {v} vs {want}");
        }
    }

    #[test]
    fn convolution_is_linear_convolution() {
        let x = [1.0, 2.0, 3.0, 0.0, -1.0];
        let h = [0.5, 0.25];
        let y = convolve(&x, &h);
        let want = [0.5, 1.25, 2.0, 0.75, -0.5];
        for (a, b) in y.iter().zip(want) {
            assert!((a - b).abs() < 1e-12);
        }
    }
}
