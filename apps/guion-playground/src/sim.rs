//! Pendulum simulation backed by garust-physics.

use core::f64::consts::TAU;

use garust::physics::world::{Body, Joint, World, STATIC};
use garust::{pga, Motor, Motor3, Pga3};
use motoreel::{record, Track};

/// User-facing pendulum parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimParams {
    pub length: f64,
    pub gravity: f64,
    pub angle_deg: f64,
}

impl Default for SimParams {
    fn default() -> Self {
        SimParams {
            length: 1.0,
            gravity: 9.81,
            angle_deg: 30.0,
        }
    }
}

/// Fixed-timestep physics integrator step (four steps per 60 fps frame).
pub const SIM_DT: f64 = 1.0 / 240.0;

/// Default export / preview frame rate.
pub const EXPORT_FPS: f64 = 60.0;

/// Default baked rollout length in seconds (~two small-angle periods at ℓ = 1 m).
pub const DEFAULT_DURATION: f64 = 4.0;

/// Baked pendulum simulation ready for scrub and export.
#[derive(Clone, Debug)]
pub struct SimulationState {
    pub params: SimParams,
    pub world: World,
    pub bodies: Vec<Body>,
    pub joints: Vec<Joint>,
    pub track: Track,
    pub duration: f64,
}

impl SimulationState {
    pub fn new(params: SimParams) -> Result<Self, String> {
        let mut state = SimulationState {
            params,
            world: World {
                gravity: [0.0, -params.gravity, 0.0],
                ground: None,
            },
            bodies: Vec::new(),
            joints: Vec::new(),
            track: Track::hold(Motor3::identity()),
            duration: DEFAULT_DURATION,
        };
        state.reset_pendulum()?;
        state.bake(DEFAULT_DURATION)?;
        Ok(state)
    }

    pub fn set_params(&mut self, params: SimParams) -> Result<(), String> {
        self.params = params;
        self.world.gravity = [0.0, -params.gravity, 0.0];
        self.reset_pendulum()?;
        self.bake(self.duration)?;
        Ok(())
    }

    pub fn reset_pendulum(&mut self) -> Result<(), String> {
        let ell = self.params.length;
        if !(ell.is_finite() && ell > 0.0) {
            return Err("length must be finite and > 0".into());
        }
        let theta = self.params.angle_deg * TAU / 360.0;
        if !theta.is_finite() {
            return Err("angle must be finite".into());
        }

        let e1 = Pga3::basis(1);
        let e2 = Pga3::basis(2);
        let orient = Motor::rotor(theta, e1 * e2);

        let mut bob = Body::ball(1.0, 0.08);
        bob.rigid.orientation = orient;
        bob.rigid.position = [ell * theta.sin(), -ell * theta.cos(), 0.0];

        self.bodies = vec![bob];
        self.joints = vec![Joint::Hinge {
            a: STATIC,
            b: 0,
            anchor_a: [0.0, 0.0, 0.0],
            anchor_b: [0.0, ell, 0.0],
            axis: [0.0, 0.0, 1.0],
        }];
        Ok(())
    }

    pub fn bake(&mut self, duration: f64) -> Result<(), String> {
        if !(duration.is_finite() && duration > 0.0) {
            return Err("duration must be finite and > 0".into());
        }
        let steps = ((duration / SIM_DT).ceil() as usize).max(1);
        let tracks = record(&self.world, &self.bodies, &self.joints, SIM_DT, steps)
            .map_err(|e| e.to_string())?;
        self.track = tracks
            .into_iter()
            .next()
            .ok_or_else(|| "pendulum rollout produced no tracks".to_string())?;
        self.duration = steps as f64 * SIM_DT;
        Ok(())
    }

    pub fn pose_at(&self, t: f64) -> Motor3 {
        self.track.eval(t.clamp(0.0, self.duration))
    }

    pub fn bob_position(&self, t: f64) -> (f64, f64, f64) {
        pga::Point::new(0.0, 0.0, 0.0)
            .transform(&self.pose_at(t))
            .to_euclidean()
    }
}
