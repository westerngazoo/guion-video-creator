//! Third-class lever / biceps model — ported from `tools/reel09.py`.

use std::collections::BTreeMap;

/// Inputs matching the reel-09 screenplay params.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeverParams {
    pub load_mass: f64,
    pub forearm: f64,
    pub insertion: f64,
    pub upper_arm: f64,
}

impl LeverParams {
    pub fn from_map(params: &BTreeMap<String, f64>) -> Option<Self> {
        let load_mass = params.get("load_mass")?;
        let forearm = params.get("forearm")?;
        let insertion = params.get("insertion")?;
        let upper_arm = params.get("upper_arm")?;
        Some(LeverParams {
            load_mass: *load_mass,
            forearm: *forearm,
            insertion: *insertion,
            upper_arm: *upper_arm,
        })
    }
}

/// Model-space outputs at elbow angle `phi` (radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeverState {
    pub phi: f64,
    pub elbow: [f64; 3],
    pub hand: [f64; 3],
    pub insertion: [f64; 3],
    pub shoulder: [f64; 3],
    pub biceps_force_n: f64,
    pub load_force_n: f64,
    pub torque_nm: f64,
}

const G: f64 = 9.81;

impl LeverParams {
    /// Closed-form state at `phi`, matching `reel09.py`.
    pub fn state_at(&self, phi: f64) -> LeverState {
        let load_force_n = self.load_mass * G;
        let torque_nm = load_force_n * self.forearm * phi.sin();
        let biceps_force_n = if self.insertion > 0.0 {
            torque_nm / self.insertion
        } else {
            0.0
        };
        let sin = phi.sin();
        let cos = phi.cos();
        LeverState {
            phi,
            elbow: [0.0, 0.0, 0.0],
            hand: [self.forearm * sin, -self.forearm * cos, 0.0],
            insertion: [self.insertion * sin, -self.insertion * cos, 0.0],
            shoulder: [0.0, self.upper_arm, 0.0],
            biceps_force_n,
            load_force_n,
            torque_nm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reel09_params() -> LeverParams {
        LeverParams {
            load_mass: 20.0,
            forearm: 0.32,
            insertion: 0.04,
            upper_arm: 0.30,
        }
    }

    #[test]
    fn biceps_force_at_90_degrees() {
        let p = reel09_params();
        let phi = std::f64::consts::FRAC_PI_2;
        let s = p.state_at(phi);
        assert!((s.torque_nm - s.load_force_n * 0.32).abs() < 1e-6);
        assert!((s.biceps_force_n - s.torque_nm / 0.04).abs() < 1e-6);
        assert!(s.biceps_force_n > 1500.0);
    }
}
