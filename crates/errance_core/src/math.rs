//! Petite géométrie 2D : le combat se joue sur le sol (x, z du monde 3D).

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const X: Vec2 = Vec2 { x: 1.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn distance(self, o: Vec2) -> f32 {
        (self - o).length()
    }

    pub fn normalize_or_zero(self) -> Vec2 {
        let l = self.length();
        if l > 1e-6 { self * (1.0 / l) } else { Vec2::ZERO }
    }

    pub fn dot(self, o: Vec2) -> f32 {
        self.x * o.x + self.y * o.y
    }

    pub fn clamp_length(self, max: f32) -> Vec2 {
        let l = self.length();
        if l > max && l > 1e-6 { self * (max / l) } else { self }
    }

    pub fn from_angle(rad: f32) -> Vec2 {
        Vec2::new(rad.cos(), rad.sin())
    }

    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    pub fn lerp(self, o: Vec2, t: f32) -> Vec2 {
        self + (o - self) * t
    }

    pub fn perp(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}
impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Vec2) {
        self.x += o.x;
        self.y += o.y;
    }
}
impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}
impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Vec2) {
        self.x -= o.x;
        self.y -= o.y;
    }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, k: f32) -> Vec2 {
        Vec2::new(self.x * k, self.y * k)
    }
}
impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

/// Le point `p` est-il dans le cône partant de `origin`, orienté `dir`,
/// de demi-angle `half_angle` (radians) et de portée `range` ?
pub fn in_cone(origin: Vec2, dir: Vec2, half_angle: f32, range: f32, p: Vec2, p_radius: f32) -> bool {
    let d = p - origin;
    let dist = d.length();
    if dist > range + p_radius {
        return false;
    }
    if dist < 1e-4 {
        return true;
    }
    let cos = d.dot(dir.normalize_or_zero()) / dist;
    cos >= half_angle.cos() - (p_radius / dist.max(0.1)).min(0.3)
}

/// Le point est-il dans le rectangle partant de `origin` le long de `dir` ?
pub fn in_line(origin: Vec2, dir: Vec2, length: f32, half_width: f32, p: Vec2, p_radius: f32) -> bool {
    let dir = dir.normalize_or_zero();
    let d = p - origin;
    let along = d.dot(dir);
    let across = d.dot(dir.perp()).abs();
    along >= -p_radius && along <= length + p_radius && across <= half_width + p_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cone_devant_pas_derriere() {
        let o = Vec2::ZERO;
        assert!(in_cone(o, Vec2::X, 0.5, 5.0, Vec2::new(3.0, 0.5), 0.0));
        assert!(!in_cone(o, Vec2::X, 0.5, 5.0, Vec2::new(-3.0, 0.0), 0.0));
        assert!(!in_cone(o, Vec2::X, 0.5, 5.0, Vec2::new(8.0, 0.0), 0.0));
    }

    #[test]
    fn ligne() {
        assert!(in_line(Vec2::ZERO, Vec2::X, 6.0, 0.5, Vec2::new(4.0, 0.3), 0.0));
        assert!(!in_line(Vec2::ZERO, Vec2::X, 6.0, 0.5, Vec2::new(4.0, 2.0), 0.0));
    }
}
