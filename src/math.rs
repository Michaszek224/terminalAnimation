use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len > 0.0 { self / len } else { self }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

/// Row-major 3x3 matrix, used only for rotations.
#[derive(Clone, Copy, Debug)]
pub struct Mat3([[f32; 3]; 3]);

impl Mat3 {
    /// Rotation about X, then Y, then Z (angles in radians).
    pub fn rotation(ax: f32, ay: f32, az: f32) -> Mat3 {
        let (sx, cx) = ax.sin_cos();
        let (sy, cy) = ay.sin_cos();
        let (sz, cz) = az.sin_cos();
        let rx = Mat3([[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]]);
        let ry = Mat3([[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]]);
        let rz = Mat3([[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]]);
        rz * ry * rx
    }
}

impl Mul for Mat3 {
    type Output = Mat3;
    fn mul(self, o: Mat3) -> Mat3 {
        let mut m = [[0.0; 3]; 3];
        for (r, row) in m.iter_mut().enumerate() {
            for (c, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[r][k] * o.0[k][c]).sum();
            }
        }
        Mat3(m)
    }
}

impl Mul<Vec3> for Mat3 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        let m = &self.0;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_preserves_length() {
        let v = Vec3::new(0.3, -1.2, 0.7);
        let r = Mat3::rotation(0.4, 1.3, -2.1) * v;
        assert!((r.length() - v.length()).abs() < 1e-5);
    }

    #[test]
    fn quarter_turn_about_z() {
        let r = Mat3::rotation(0.0, 0.0, std::f32::consts::FRAC_PI_2) * Vec3::new(1.0, 0.0, 0.0);
        assert!((r - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
    }
}
