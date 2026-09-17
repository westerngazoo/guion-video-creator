//! reel39 — press de hombro: Smith contra militar (máquina humana 2D).

const G: f64 = 9.81;

/// Brazo superior / antebrazo (m) y rama IK (hombro → codo).
const L_UPPER: f64 = 0.338_52;
const L_FORE: f64 = 0.312_48;
const BRANCH: (f64, f64) = (1.0, -0.5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variante {
    Militar,
    Smith,
}

impl Variante {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "militar" => Some(Variante::Militar),
            "smith" => Some(Variante::Smith),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose2 {
    pub l5: (f64, f64),
    pub hombro: (f64, f64),
    pub codo: (f64, f64),
    pub mano: (f64, f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reel39 {
    pub carga: f64,
    pub masa: f64,
    pub x_militar: f64,
    pub x_smith: f64,
    pub alcance: f64,
    pub hombro_y: f64,
    pub l5_y: f64,
}

impl Default for Reel39 {
    fn default() -> Self {
        Reel39 {
            carga: 40.0,
            masa: 80.0,
            x_militar: 0.12,
            x_smith: 0.1,
            alcance: 0.631_47,
            hombro_y: 1.454,
            l5_y: 0.95,
        }
    }
}

impl Reel39 {
    pub fn fuerza_carga(&self) -> f64 {
        self.carga * G
    }

    /// Posición de la mano en el plano sagital (m).
    pub fn mano(&self, variante: Variante, u: f64) -> (f64, f64) {
        let y = match variante {
            // Smith: recorrido vertical ligeramente menor (publicado).
            Variante::Smith => self.hombro_y + (self.alcance - 0.008) * u,
            Variante::Militar => self.hombro_y + self.alcance * u,
        };
        let x = match variante {
            Variante::Smith => self.x_smith,
            Variante::Militar => {
                if u <= 0.25 {
                    self.x_militar
                } else {
                    let t = (u - 0.25) / 0.75;
                    let knots = [
                        (0.0, self.x_militar),
                        (0.333_333_333_333_333_3, 0.099_961_926_761_989_78),
                        (0.666_666_666_666_666_6, 0.049_980_963_380_994_89),
                        (1.0, 0.0),
                    ];
                    lerp_knots(t, &knots)
                }
            }
        };
        (x, y)
    }

    /// IK de dos círculos con rama explícita (`FISICA.md` §2).
    pub fn ik_codo(&self, hombro: (f64, f64), mano: (f64, f64)) -> (f64, f64) {
        ik_two_circle(hombro, mano, L_UPPER, L_FORE, BRANCH)
    }

    pub fn pose(&self, variante: Variante, u: f64) -> Pose2 {
        let hombro = (0.0, self.hombro_y);
        let mano = self.mano(variante, u);
        let codo = self.ik_codo(hombro, mano);
        Pose2 {
            l5: (0.0, self.l5_y),
            hombro,
            codo,
            mano,
        }
    }

    /// Torque en hombro por la carga vertical (N·m).
    pub fn tau_hombro(&self, pose: &Pose2) -> f64 {
        self.fuerza_carga() * pose.mano.0.abs()
    }

    /// Torque en codo por la carga vertical (N·m).
    pub fn tau_codo(&self, pose: &Pose2) -> f64 {
        self.fuerza_carga() * (pose.mano.0 - pose.codo.0).abs()
    }

    /// Trabajo de la carga en su línea vertical (J).
    pub fn trabajo_carga(&self) -> f64 {
        self.fuerza_carga() * self.alcance
    }

    pub fn hombro_grados(&self, pose: &Pose2) -> f64 {
        let dx = pose.codo.0 - pose.hombro.0;
        let dy = pose.codo.1 - pose.hombro.1;
        180.0 - dx.atan2(dy).to_degrees()
    }

    pub fn codo_grados(&self, pose: &Pose2) -> f64 {
        let a1 = limb_angle(pose.hombro, pose.codo);
        let a2 = limb_angle(pose.codo, pose.mano);
        180.0 + normalize_deg(a2 - a1)
    }

    /// Torque medio en hombro para barra con offset horizontal `x` (m).
    pub fn tau_hombro_offset(&self, x: f64) -> f64 {
        self.fuerza_carga() * x.abs()
    }
}

fn limb_angle(from: (f64, f64), to: (f64, f64)) -> f64 {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    dx.atan2(dy).to_degrees()
}

fn normalize_deg(a: f64) -> f64 {
    let mut x = a % 360.0;
    if x > 180.0 {
        x -= 360.0;
    } else if x < -180.0 {
        x += 360.0;
    }
    x
}

fn lerp_knots(t: f64, knots: &[(f64, f64)]) -> f64 {
    let t = t.clamp(0.0, 1.0);
    for window in knots.windows(2) {
        let (t0, v0) = window[0];
        let (t1, v1) = window[1];
        if t <= t1 {
            if (t1 - t0).abs() < 1e-12 {
                return v1;
            }
            let k = (t - t0) / (t1 - t0);
            return v0 + k * (v1 - v0);
        }
    }
    knots.last().map(|k| k.1).unwrap_or(0.0)
}

fn ik_two_circle(
    shoulder: (f64, f64),
    hand: (f64, f64),
    l1: f64,
    l2: f64,
    branch: (f64, f64),
) -> (f64, f64) {
    let (sx, sy) = shoulder;
    let (hx, hy) = hand;
    let dx = hx - sx;
    let dy = hy - sy;
    let d = (dx * dx + dy * dy).sqrt();
    if d < 1e-12 {
        return (sx, sy - l1);
    }
    let a = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let h2 = (l1 * l1 - a * a).max(0.0);
    let h = h2.sqrt();
    let mx = sx + a * dx / d;
    let my = sy + a * dy / d;
    let px = -dy / d;
    let py = dx / d;
    let e1 = (mx + h * px, my + h * py);
    let e2 = (mx - h * px, my - h * py);
    let score = |p: (f64, f64)| (p.0 - mx) * branch.0 + (p.1 - my) * branch.1;
    if score(e1) >= score(e2) {
        e1
    } else {
        e2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smith_hombro_flat() {
        let r = Reel39::default();
        for u in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let p = r.pose(Variante::Smith, u);
            let t = r.tau_hombro(&p);
            assert!((t - 39.24).abs() < 0.5, "u={u} tau={t}");
        }
    }
}
