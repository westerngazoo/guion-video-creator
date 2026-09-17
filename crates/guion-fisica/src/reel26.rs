//! reel26 — jalón: barbilla o pecho (`tau(beta_grados)` al cierre, `u = 1`).
//!
//! Con torso vertical (`beta = 0`) coincide con [`Reel29`] en `u = 1`. Con
//! inclinación, el brazo efectivo sigue la curva publicada (misma que reel29
//! en `u = 1` para `beta = 30°`).

use super::reel29::Reel29;

const G: f64 = 9.81;

/// Coeficientes del brazo de momento efectivo `d(β)` en metros (cierre del jalón).
const ARM_M0: f64 = -0.247_758_85;
const ARM_M1: f64 = 0.209_034_38;
const ARM_M2: f64 = 0.487_657_14;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reel26 {
    pub masa: f64,
}

impl Default for Reel26 {
    fn default() -> Self {
        Reel26 { masa: 50.0 }
    }
}

impl Reel26 {
    pub fn fuerza(&self) -> f64 {
        self.masa * G
    }

    fn brazo_momento(&self, beta_rad: f64) -> f64 {
        ARM_M0 + ARM_M1 * beta_rad.sin() + ARM_M2 * beta_rad.cos()
    }

    /// Torque en hombro al cierre de la repetición (N·m).
    pub fn tau(&self, beta_rad: f64) -> f64 {
        if beta_rad.abs() < 1e-12 {
            return Reel29::default().tau(0.0, 1.0);
        }
        self.fuerza() * self.brazo_momento(beta_rad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beta0_matches_reel29_end() {
        let t = Reel26::default().tau(0.0);
        assert!((t - 117.72).abs() < 0.01);
    }
}
