//! Closed-form motion for the three stages. The same formulas the board
//! writes, so a frame cannot show a number the class did not derive. `G`
//! matches the classroom labs (akademos lesson-core) so the video and the
//! practice agree to the digit.

pub const G: f64 = 9.8;

/// The cart at time `tau` after the brakes go on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Carro {
    pub x: f64,
    pub y: f64,
    pub v: f64,
    /// Still braking on the road (the deceleration arrow is shown).
    pub frenando: bool,
}

/// Braking from `v0` at `a` towards a cliff at `borde` that drops `fondo`.
#[derive(Debug, Clone, Copy)]
pub struct Frenado {
    pub v0: f64,
    pub a: f64,
    pub borde: f64,
    pub fondo: f64,
}

impl Frenado {
    /// The stopping distance d = v0² / (2a).
    pub fn distancia(&self) -> f64 {
        self.v0 * self.v0 / (2.0 * self.a)
    }

    pub fn cae(&self) -> bool {
        self.distancia() > self.borde
    }

    /// When it reaches the edge, and how fast: v² = v0² − 2a·borde.
    fn al_borde(&self) -> (f64, f64) {
        let v = (self.v0 * self.v0 - 2.0 * self.a * self.borde)
            .max(0.0)
            .sqrt();
        ((self.v0 - v) / self.a, v)
    }

    /// How long the motion lasts: to the stop, or to the bottom of the cliff.
    pub fn duracion(&self) -> f64 {
        if self.cae() {
            self.al_borde().0 + (2.0 * self.fondo / G).sqrt()
        } else {
            self.v0 / self.a
        }
    }

    pub fn en(&self, tau: f64) -> Carro {
        let tau = tau.clamp(0.0, self.duracion());
        if self.cae() {
            let (tb, vb) = self.al_borde();
            if tau > tb {
                // Off the edge the brakes grip nothing: free fall.
                let s = tau - tb;
                return Carro {
                    x: self.borde + vb * s,
                    y: -(0.5 * G * s * s).min(self.fondo),
                    v: vb,
                    frenando: false,
                };
            }
        }
        let t = tau.min(self.v0 / self.a);
        Carro {
            x: self.v0 * t - 0.5 * self.a * t * t,
            y: 0.0,
            v: self.v0 - self.a * t,
            frenando: t < self.v0 / self.a,
        }
    }
}

/// A shot from the origin at `th` degrees and `v0` m/s.
#[derive(Debug, Clone, Copy)]
pub struct Tiro {
    pub th: f64,
    pub v0: f64,
}

impl Tiro {
    pub fn vx(&self) -> f64 {
        self.v0 * self.th.to_radians().cos()
    }
    pub fn vy(&self) -> f64 {
        self.v0 * self.th.to_radians().sin()
    }
    pub fn vuelo(&self) -> f64 {
        2.0 * self.vy() / G
    }
    /// R = v0²·sin(2θ)/g.
    pub fn alcance(&self) -> f64 {
        self.vx() * self.vuelo()
    }
    pub fn en(&self, t: f64) -> [f64; 2] {
        [self.vx() * t, self.vy() * t - 0.5 * G * t * t]
    }
    /// When the ball meets a wall at `x` lower than `alto`, if it does.
    pub fn choca(&self, x: f64, alto: f64) -> Option<f64> {
        let t = x / self.vx();
        (self.alcance() >= x && self.en(t)[1] < alto).then_some(t)
    }
}

/// A block released from rest at the top of a ramp.
#[derive(Debug, Clone, Copy)]
pub struct Plano {
    pub th: f64,
    pub mu: f64,
    /// The distance the block can slide, in metres.
    pub recorrido: f64,
}

impl Plano {
    /// a = g·(sinθ − μ·cosθ); never negative: friction only holds.
    pub fn a(&self) -> f64 {
        let th = self.th.to_radians();
        (G * (th.sin() - self.mu * th.cos())).max(0.0)
    }
    pub fn duracion(&self) -> f64 {
        let a = self.a();
        if a > 0.0 {
            (2.0 * self.recorrido / a).sqrt()
        } else {
            1.5
        }
    }
    /// Distance slid at time `t`.
    pub fn s(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, self.duracion());
        (0.5 * self.a() * t * t).min(self.recorrido)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn braking_matches_the_board() {
        let f = Frenado {
            v0: 8.0,
            a: 2.0,
            borde: 20.0,
            fondo: 3.6,
        };
        assert!(close(f.distancia(), 16.0));
        assert!(close(f.duracion(), 4.0));
        let end = f.en(10.0);
        assert!(close(end.x, 16.0) && close(end.v, 0.0) && !end.frenando);
        assert!(close(Frenado { a: 4.0, ..f }.distancia(), 8.0));
    }

    #[test]
    fn too_soft_a_brake_goes_over_the_edge_and_falls() {
        let f = Frenado {
            v0: 8.0,
            a: 1.0,
            borde: 20.0,
            fondo: 3.6,
        };
        assert!(f.cae() && close(f.distancia(), 32.0));
        let (tb, vb) = f.al_borde();
        assert!(close(f.en(tb).x, 20.0));
        assert!(close(vb * vb, 64.0 - 40.0));
        assert!(close(f.en(f.duracion()).y, -3.6));
    }

    #[test]
    fn complementary_angles_land_together() {
        let a = Tiro { th: 60.0, v0: 15.0 }.alcance();
        let b = Tiro { th: 30.0, v0: 15.0 }.alcance();
        assert!((a - b).abs() < 1e-9 && (a - 19.88).abs() < 0.01);
    }

    #[test]
    fn the_wall_stops_a_low_shot() {
        assert!(Tiro { th: 45.0, v0: 17.5 }.choca(17.5, 9.0).is_some());
        assert!(Tiro { th: 55.0, v0: 18.0 }.choca(17.5, 9.0).is_none());
    }

    #[test]
    fn friction_holds_at_ten_degrees_and_yields_at_thirty_five() {
        assert_eq!(
            Plano {
                th: 10.0,
                mu: 0.2,
                recorrido: 8.0
            }
            .a(),
            0.0
        );
        let a = Plano {
            th: 35.0,
            mu: 0.2,
            recorrido: 8.0,
        }
        .a();
        assert!((a - 4.02).abs() < 0.01);
    }
}
