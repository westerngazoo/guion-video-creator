use guion_core::Style as SpStyle;
use guion_core::{Format, Meta};
use motoreel::Style;

use guion_brand::Theme;

pub fn build(spec: &SpStyle, theme: &Theme, meta: &Meta, heat_value: Option<f64>) -> Style {
    let stroke = theme.stroke_color(&spec.stroke, heat_value);
    let view = crate::camera::view_for(meta);
    let canvas_h = match meta.format {
        Format::Vertical => 1920.0,
    };
    let width = spec.width / canvas_h * view.1;
    Style {
        stroke,
        width,
        alpha: 1.0,
    }
}
