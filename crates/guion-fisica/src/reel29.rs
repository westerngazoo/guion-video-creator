//! reel29 / reel26 — jalón en polea (`beta_grados`, `u`).
//!
//! En `beta = 0` la mano recorre una línea vertical en `x = POLEA.x`; el torque
//! en hombro es `T·d` con `d = POLEA.x` y `T = M·g`. El largo de cable cuadra
//! con `|polea − mano|`.

const G: f64 = 9.81;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reel29 {
    pub masa: f64,
    pub polea: (f64, f64),
    /// Mano al inicio del tirón (`u = 0`).
    pub mano_ini: (f64, f64),
    /// Mano al cierre (`u = 1`).
    pub mano_fin: (f64, f64),
}

impl Default for Reel29 {
    fn default() -> Self {
        Reel29 {
            masa: 50.0,
            polea: (0.24, 1.6),
            // `y0` fija `largo_cable(u=0)` con la polea en `(0.24, 1.6)`.
            mano_ini: (0.24, 1.080_211_278_642_769),
            mano_fin: (0.24, 0.4),
        }
    }
}

impl Reel29 {
    pub fn tension(&self) -> f64 {
        self.masa * G
    }

    /// Posición de la mano en el marco del hombro (origen).
    pub fn mano(&self, beta_rad: f64, u: f64) -> (f64, f64) {
        let (x0, y0) = self.mano_ini;
        let (x1, y1) = self.mano_fin;
        let x = x0 + (x1 - x0) * u;
        let y = y0 + (y1 - y0) * u;
        let c = beta_rad.cos();
        let s = beta_rad.sin();
        (x * c + y * s, -x * s + y * c)
    }

    /// Longitud del tramo polea–mano (m).
    pub fn largo_cable(&self, beta_rad: f64, u: f64) -> f64 {
        let (hx, hy) = self.mano(beta_rad, u);
        let dx = self.polea.0 - hx;
        let dy = self.polea.1 - hy;
        (dx * dx + dy * dy).sqrt()
    }

    /// Torque en hombro (N·m): `|r × F⃗|` con `F⃗` a lo largo del cable.
    pub fn tau(&self, beta_rad: f64, u: f64) -> f64 {
        let (hx, hy) = self.mano(beta_rad, u);
        let dx = self.polea.0 - hx;
        let dy = self.polea.1 - hy;
        let n = (dx * dx + dy * dy).sqrt();
        let t = self.tension();
        (t * (hx * dy - hy * dx) / n).abs()
    }

    /// Recorrido de la carga a lo largo de su línea de acción (m).
    pub fn recorrido(&self, beta_rad: f64) -> f64 {
        self.largo_cable(beta_rad, 1.0) - self.largo_cable(beta_rad, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beta0_tau_constant() {
        let r = Reel29::default();
        for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert!((r.tau(0.0, u) - 117.72).abs() < 0.01);
        }
    }

    #[test]
    fn beta0_cable_lengths() {
        let r = Reel29::default();
        assert!((r.largo_cable(0.0, 0.0) - 0.519_788_721_357_231).abs() < 1e-6);
        assert!((r.largo_cable(0.0, 1.0) - 1.2).abs() < 1e-6);
        assert!((r.recorrido(0.0) - 0.680_211_278_642_769_2).abs() < 1e-6);
    }
}
