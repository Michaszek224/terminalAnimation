use std::fmt::Write as _;

use clap::ValueEnum;

use crate::math::Vec3;
use crate::palette::{ColorScheme, Rgb};
use crate::shapes::Tri;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum RenderMode {
    /// Solid surfaces shaded with ASCII characters.
    Fill,
    /// Edges drawn with Braille dots (2x4 sub-pixels per character).
    Wire,
}

/// Brightness ramp, darkest to brightest.
const RAMP: &[char] = &['.', ',', '-', '~', ':', ';', '=', '!', '*', '#', '$', '@'];

/// Braille dot bit for sub-pixel (column 0..2, row 0..4) inside a cell.
const BRAILLE: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

/// Distance from the camera to the origin; objects live inside the unit sphere.
const CAMERA: f32 = 5.0;
/// How much perspective can enlarge the nearest point of a unit-radius object.
const PERSPECTIVE_GROWTH: f32 = CAMERA / (CAMERA - 1.0);

const HUD_COLOR: Rgb = (130, 130, 140);

struct Projector {
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
}

impl Projector {
    /// `aspect` = horizontal units per vertical unit needed for circles to look round.
    fn new(w: f32, h: f32, aspect: f32, size: f32) -> Self {
        let scale = (h / 2.0).min(w / (2.0 * aspect)) * 0.92 * size / PERSPECTIVE_GROWTH;
        Self { cx: w / 2.0, cy: h / 2.0, sx: scale * aspect, sy: scale }
    }

    /// Screen x/y, keeping the view-space z for depth tests.
    fn project(&self, v: Vec3) -> Vec3 {
        let k = CAMERA / (v.z + CAMERA);
        Vec3::new(self.cx + v.x * k * self.sx, self.cy - v.y * k * self.sy, v.z)
    }
}

pub struct Renderer {
    w: usize,
    h: usize,
    /// Reserve the bottom row for the status line.
    pub hud: bool,
    chars: Vec<char>,
    colors: Vec<Option<Rgb>>,
    zbuf: Vec<f32>,
    dots: Vec<u8>,
    out: String,
}

impl Renderer {
    pub fn new(w: usize, h: usize) -> Self {
        let mut r = Self {
            w: 0,
            h: 0,
            hud: false,
            chars: Vec::new(),
            colors: Vec::new(),
            zbuf: Vec::new(),
            dots: Vec::new(),
            out: String::new(),
        };
        r.resize(w, h);
        r
    }

    pub fn resize(&mut self, w: usize, h: usize) {
        (self.w, self.h) = (w.max(1), h.max(1));
        let n = self.w * self.h;
        self.chars = vec![' '; n];
        self.colors = vec![None; n];
        self.zbuf = vec![f32::INFINITY; n];
        self.dots = vec![0; n];
    }

    fn scene_rows(&self) -> usize {
        if self.hud && self.h > 1 { self.h - 1 } else { self.h }
    }

    fn clear(&mut self) {
        self.chars.fill(' ');
        self.colors.fill(None);
        self.zbuf.fill(f32::INFINITY);
        self.dots.fill(0);
    }

    /// Z-buffered triangle rasterization at one sample per character cell.
    pub fn fill(&mut self, verts: &[Vec3], normals: &[Vec3], tris: &[Tri], scheme: ColorScheme, t: f32, size: f32) {
        self.clear();
        let (w, rows) = (self.w, self.scene_rows());
        // Terminal cells are roughly twice as tall as they are wide.
        let proj = Projector::new(w as f32, rows as f32, 2.0, size);
        let pts: Vec<Vec3> = verts.iter().map(|&v| proj.project(v)).collect();
        let light = Vec3::new(-0.5, 0.6, -0.7).normalize();
        let halfway = (light + Vec3::new(0.0, 0.0, -1.0)).normalize();

        for &[(a, na), (b, nb), (c, nc)] in tris {
            let (p0, p1, p2) = (pts[a], pts[b], pts[c]);
            let area = edge(p0, p1, p2.x, p2.y);
            if area.abs() < 1e-6 {
                continue;
            }
            let x0 = (p0.x.min(p1.x).min(p2.x).floor() as i64).max(0);
            let x1 = (p0.x.max(p1.x).max(p2.x).ceil() as i64).min(w as i64 - 1);
            let y0 = (p0.y.min(p1.y).min(p2.y).floor() as i64).max(0);
            let y1 = (p0.y.max(p1.y).max(p2.y).ceil() as i64).min(rows as i64 - 1);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let b0 = edge(p1, p2, px, py) / area;
                    let b1 = edge(p2, p0, px, py) / area;
                    let b2 = 1.0 - b0 - b1;
                    if b0 < -1e-4 || b1 < -1e-4 || b2 < -1e-4 {
                        continue;
                    }
                    let i = y as usize * w + x as usize;
                    let z = b0 * p0.z + b1 * p1.z + b2 * p2.z;
                    if z >= self.zbuf[i] {
                        continue;
                    }
                    self.zbuf[i] = z;
                    let n = (normals[na] * b0 + normals[nb] * b1 + normals[nc] * b2).normalize();
                    let diffuse = n.dot(light).max(0.0);
                    let specular = n.dot(halfway).max(0.0).powi(20);
                    let lum = (0.08 + 0.72 * diffuse + 0.45 * specular).min(1.0);
                    self.chars[i] = RAMP[(lum * (RAMP.len() - 1) as f32).round() as usize];
                    self.colors[i] = scheme.color(lum, hue_hint(t, x as usize, y as usize, w, rows));
                }
            }
        }
    }

    /// Wireframe on a Braille sub-pixel canvas; nearer edges are drawn brighter.
    pub fn wire(&mut self, verts: &[Vec3], edges: &[(usize, usize)], scheme: ColorScheme, t: f32, size: f32) {
        self.clear();
        let (w, rows) = (self.w, self.scene_rows());
        let (pw, ph) = (w * 2, rows * 4);
        // Braille dots are close to square, so no aspect correction is needed.
        let proj = Projector::new(pw as f32, ph as f32, 1.0, size);

        for &(a, b) in edges {
            let (p, q) = (proj.project(verts[a]), proj.project(verts[b]));
            let steps = (q.x - p.x).abs().max((q.y - p.y).abs()).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let f = s as f32 / steps as f32;
                let (x, y) = ((p.x + (q.x - p.x) * f).floor(), (p.y + (q.y - p.y) * f).floor());
                if x < 0.0 || y < 0.0 || x >= pw as f32 || y >= ph as f32 {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                let i = (y / 4) * w + x / 2;
                self.dots[i] |= BRAILLE[y % 4][x % 2];
                self.zbuf[i] = self.zbuf[i].min(p.z + (q.z - p.z) * f);
            }
        }

        for i in 0..w * rows {
            if self.dots[i] != 0 {
                self.chars[i] = char::from_u32(0x2800 + self.dots[i] as u32).unwrap_or(' ');
                // z is in -1..1 with -1 nearest the camera.
                let lum = 1.0 - (self.zbuf[i] + 1.0) / 2.0 * 0.7;
                self.colors[i] = scheme.color(lum, hue_hint(t, i % w, i / w, w, rows));
            }
        }
    }

    pub fn status(&mut self, text: &str) {
        if !self.hud || self.h < 2 {
            return;
        }
        let start = (self.h - 1) * self.w;
        let mut chars = text.chars();
        for i in start..start + self.w {
            self.chars[i] = chars.next().unwrap_or(' ');
            self.colors[i] = Some(HUD_COLOR);
        }
    }

    /// Full frame as one string, with ANSI truecolor codes when `color` is set.
    pub fn compose(&mut self, color: bool) -> &str {
        self.out.clear();
        let mut current: Option<Rgb> = None;
        for y in 0..self.h {
            for x in 0..self.w {
                let i = y * self.w + x;
                if let (true, Some(rgb)) = (color, self.colors[i])
                    && current != Some(rgb)
                {
                    let _ = write!(self.out, "\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2);
                    current = Some(rgb);
                }
                self.out.push(self.chars[i]);
            }
            if y + 1 < self.h {
                self.out.push_str("\r\n");
            }
        }
        if color {
            self.out.push_str("\x1b[0m");
        }
        &self.out
    }
}

/// Signed area test: positive on one side of the edge a→b, negative on the other.
fn edge(a: Vec3, b: Vec3, px: f32, py: f32) -> f32 {
    (px - a.x) * (b.y - a.y) - (py - a.y) * (b.x - a.x)
}

/// Diagonal rainbow sweep that drifts over time.
fn hue_hint(t: f32, x: usize, y: usize, w: usize, h: usize) -> f32 {
    t * 0.06 + x as f32 / w as f32 * 0.5 + y as f32 / h as f32 * 0.3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::{Model, ShapeKind};

    fn cube() -> crate::shapes::Mesh {
        match ShapeKind::Cube.build() {
            Model::Mesh(m) => m,
            Model::Tesseract(_) => unreachable!(),
        }
    }

    fn drawn(r: &Renderer) -> Vec<(usize, usize)> {
        (0..r.w * r.h).filter(|&i| r.chars[i] != ' ').map(|i| (i % r.w, i / r.w)).collect()
    }

    #[test]
    fn fill_draws_centered_shape_inside_bounds() {
        let m = cube();
        let mut r = Renderer::new(60, 30);
        r.fill(&m.verts, &m.normals, &m.tris, ColorScheme::Plain, 0.0, 1.0);
        let cells = drawn(&r);
        assert!(cells.len() > 100);
        let cx = cells.iter().map(|c| c.0).sum::<usize>() as f32 / cells.len() as f32;
        let cy = cells.iter().map(|c| c.1).sum::<usize>() as f32 / cells.len() as f32;
        assert!((cx - 30.0).abs() < 2.0 && (cy - 15.0).abs() < 2.0, "center {cx},{cy}");
        assert!(!cells.iter().any(|&(x, y)| x == 0 || y == 0 || x == 59 || y == 29));
    }

    #[test]
    fn wire_uses_braille_and_leaves_hud_row_free() {
        let m = cube();
        let mut r = Renderer::new(40, 20);
        r.hud = true;
        r.wire(&m.verts, &m.edges, ColorScheme::Fire, 0.0, 1.0);
        let cells = drawn(&r);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|&(x, y)| ('\u{2801}'..='\u{28FF}').contains(&r.chars[y * 40 + x]) && y < 19));
    }

    #[test]
    fn compose_emits_one_line_per_row() {
        let mut r = Renderer::new(10, 4);
        r.hud = true;
        r.status("hello");
        let out = r.compose(false).to_string();
        assert_eq!(out.matches("\r\n").count(), 3);
        assert!(out.contains("hello"));
    }
}
