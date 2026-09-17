use garust::Motor3;
use guion_core::{Camera as SpCamera, CameraKind, Format, Meta};
use motoreel::Camera;

use crate::error::AssembleError;

pub fn build(cam: &SpCamera) -> Result<Camera, AssembleError> {
    let pose = Motor3::translator(
        cam.pose.translate[0],
        cam.pose.translate[1],
        cam.pose.translate[2],
    );
    Ok(match cam.kind {
        CameraKind::Orthographic => Camera::orthographic(pose),
        CameraKind::Perspective => Camera::pinhole(pose, 1.0),
    })
}

/// Image-space view window for a format preset (width, height), y-up.
pub fn view_for(meta: &Meta) -> (f64, f64) {
    match meta.format {
        Format::Vertical => (1.8, 3.2),
    }
}
