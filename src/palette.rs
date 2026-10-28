use clap::ValueEnum;

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColorScheme {
    Rainbow,
    Fire,
    Ocean,
    Matrix,
    Neon,
    Mono,
    /// No ANSI colors at all, characters only.
    #[value(name = "none")]
    Plain,
}

impl ColorScheme {
    /// `lum` is surface brightness in 0..=1; `hint` is a free parameter (position/time) used by rainbow.
    pub fn color(self, lum: f32, hint: f32) -> Option<Rgb> {
        let lum = lum.clamp(0.0, 1.0);
        Some(match self {
            ColorScheme::Rainbow => hsv(hint, 0.75, 0.3 + 0.7 * lum),
            ColorScheme::Fire => gradient(
                &[(0.0, (70, 0, 0)), (0.35, (190, 30, 0)), (0.65, (255, 130, 0)), (0.85, (255, 220, 70)), (1.0, (255, 255, 220))],
                lum,
            ),
            ColorScheme::Ocean => gradient(
                &[(0.0, (0, 25, 70)), (0.45, (0, 100, 200)), (0.8, (0, 215, 230)), (1.0, (220, 255, 255))],
                lum,
            ),
            ColorScheme::Matrix => gradient(&[(0.0, (0, 45, 0)), (0.6, (0, 200, 60)), (1.0, (190, 255, 190))], lum),
            ColorScheme::Neon => gradient(
                &[(0.0, (45, 0, 90)), (0.45, (210, 0, 200)), (0.8, (0, 220, 255)), (1.0, (255, 255, 255))],
                lum,
            ),
            ColorScheme::Mono => {
                let g = (70.0 + 185.0 * lum) as u8;
                (g, g, g)
            }
            ColorScheme::Plain => return None,
        })
    }
}

fn gradient(stops: &[(f32, Rgb)], t: f32) -> Rgb {
    let i = stops.iter().position(|&(pos, _)| pos >= t).unwrap_or(stops.len() - 1).max(1);
    let ((p0, c0), (p1, c1)) = (stops[i - 1], stops[i]);
    let f = ((t - p0) / (p1 - p0)).clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * f) as u8;
    (mix(c0.0, c1.0), mix(c0.1, c1.1), mix(c0.2, c1.2))
}

/// Hue wraps around, so any `h` is valid.
fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to = |ch: f32| ((ch + m) * 255.0) as u8;
    (to(r), to(g), to(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_hits_endpoints() {
        let stops = [(0.0, (0, 0, 0)), (0.5, (100, 100, 100)), (1.0, (200, 200, 200))];
        assert_eq!(gradient(&stops, 0.0), (0, 0, 0));
        assert_eq!(gradient(&stops, 1.0), (200, 200, 200));
        assert_eq!(gradient(&stops, 0.75), (150, 150, 150));
    }

    #[test]
    fn hsv_primary_colors() {
        assert_eq!(hsv(0.0, 1.0, 1.0), (255, 0, 0));
        assert_eq!(hsv(1.0 / 3.0, 1.0, 1.0).1, 255);
        assert_eq!(hsv(2.0 / 3.0, 1.0, 1.0).2, 255);
    }
}
