use guion_core::{EaseName, Motion, Screenplay};

/// Scene length in seconds. M1 uses the hook window when present.
pub fn duration(sp: &Screenplay) -> f64 {
    if let Some(hook) = &sp.hook {
        hook.at.end
    } else {
        let hold = sp.motion.hold.as_ref();
        let hs = hold.map(|h| h.start).unwrap_or(0.0);
        let he = hold.map(|h| h.end).unwrap_or(0.0);
        (hs + he + 1.0).max(1.0)
    }
}

/// Drive value at time `t` along the screenplay `[motion]` timeline.
///
/// `hold.start` / `hold.end` are dwell durations in seconds at each end;
/// the eased ramp occupies the middle (reel-09: 1.0 s in, 0.87 s out).
pub fn driven_value(m: &Motion, t: f64, total: f64) -> f64 {
    if total <= 0.0 {
        return m.from;
    }
    let hold = m.hold.as_ref();
    let dwell_start = hold.map(|h| h.start).unwrap_or(0.0);
    let dwell_end = hold.map(|h| h.end).unwrap_or(0.0);
    let ramp_start = dwell_start.min(total);
    let ramp_end = (total - dwell_end).max(ramp_start);

    if t <= ramp_start {
        m.from
    } else if t >= ramp_end || ramp_end <= ramp_start {
        m.to
    } else {
        let u = (t - ramp_start) / (ramp_end - ramp_start);
        let eased = apply_ease(m.ease, u);
        m.from + eased * (m.to - m.from)
    }
}

/// Evaluate `phi` for the golden lever screenplay at time `t`.
pub fn phi_at(sp: &Screenplay, t: f64) -> f64 {
    let total = duration(sp);
    driven_value(&sp.motion, t, total)
}

fn apply_ease(ease: EaseName, u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    match ease {
        EaseName::Linear => u,
        EaseName::Smoothstep => u * u * (3.0 - 2.0 * u),
        EaseName::Smootherstep => u * u * u * (u * (u * 6.0 - 15.0) + 10.0),
        EaseName::EaseIn => u * u,
        EaseName::EaseOut => 1.0 - (1.0 - u) * (1.0 - u),
        EaseName::EaseInOut => {
            if u < 0.5 {
                2.0 * u * u
            } else {
                1.0 - (-2.0 * u + 2.0).powi(2) / 2.0
            }
        }
    }
}
