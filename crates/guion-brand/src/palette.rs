use motoreel::Rgb;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteFile {
    pub colors: Colors,
    #[serde(default)]
    pub heat: Vec<HeatStop>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Colors {
    pub skin: [u8; 3],
    pub ink: [u8; 3],
    pub paper: [u8; 3],
    pub accent: [u8; 3],
    pub accent2: [u8; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeatStop {
    pub t: f64,
    pub rgb: [u8; 3],
}

impl PaletteFile {
    pub fn parse(src: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(src)
    }
}

pub fn rgb(arr: [u8; 3]) -> Rgb {
    Rgb {
        r: arr[0],
        g: arr[1],
        b: arr[2],
    }
}

pub fn heat_color(stops: &[HeatStop], t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    if stops.is_empty() {
        return Rgb::WHITE;
    }
    if t <= stops[0].t {
        return rgb(stops[0].rgb);
    }
    for w in stops.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t <= b.t {
            let k = (t - a.t) / (b.t - a.t);
            return Rgb {
                r: lerp_u8(a.rgb[0], b.rgb[0], k),
                g: lerp_u8(a.rgb[1], b.rgb[1], k),
                b: lerp_u8(a.rgb[2], b.rgb[2], k),
            };
        }
    }
    rgb(stops.last().unwrap().rgb)
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}
