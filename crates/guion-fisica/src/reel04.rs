//! reel04 — curl barra vs polea baja (`tools/reel04.py`).

const G: f64 = 9.81;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Barra,
    Polea,
}

impl Modo {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "barra" => Some(Modo::Barra),
            "polea" => Some(Modo::Polea),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reel04 {
    pub masa: f64,
    pub lf: f64,
    pub polea: (f64, f64),
}

impl Default for Reel04 {
    fn default() -> Self {
        Reel04 {
            masa: 20.0,
            lf: 0.32,
            polea: (0.42, -0.85),
        }
    }
}

impl Reel04 {
    pub fn fuerza(&self) -> f64 {
        self.masa * G
    }

    fn mano(&self, phi: f64) -> (f64, f64) {
        (self.lf * phi.sin(), -self.lf * phi.cos())
    }

    fn direccion(&self, phi: f64, modo: Modo) -> (f64, f64) {
        match modo {
            Modo::Barra => (0.0, -1.0),
            Modo::Polea => {
                let (hx, hy) = self.mano(phi);
                let dx = self.polea.0 - hx;
                let dy = self.polea.1 - hy;
                let n = (dx * dx + dy * dy).sqrt();
                (dx / n, dy / n)
            }
        }
    }

    fn brazo_momento(&self, phi: f64, modo: Modo) -> f64 {
        let (hx, hy) = self.mano(phi);
        let (ux, uy) = self.direccion(phi, modo);
        let t = -(hx * ux + hy * uy);
        let px = hx + t * ux;
        let py = hy + t * uy;
        (px * px + py * py).sqrt()
    }

    /// Torque en el codo (N·m), igual que `tau(phi, modo)` en reel04.py.
    pub fn tau(&self, phi: f64, modo: Modo) -> f64 {
        self.fuerza() * self.brazo_momento(phi, modo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuerza_matches_python() {
        assert!((Reel04::default().fuerza() - 196.2).abs() < 1e-9);
    }

    #[test]
    fn barra_zero_at_phi_zero() {
        assert!(Reel04::default().tau(0.0, Modo::Barra).abs() < 1e-9);
    }

    #[test]
    fn polea_nonzero_at_phi_zero() {
        let t = Reel04::default().tau(0.0, Modo::Polea);
        assert!((t - 38.99398955962976).abs() / t < 0.02);
    }
}
