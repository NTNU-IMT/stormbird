
use std::f64::consts::PI;

pub struct EllipticalWing {
    pub aspect_ratio: f64,
}

impl EllipticalWing {
    #[inline(always)]
    /// Function that computes the lift-induced angle of attack according to elliptic wing theory, based
    /// only on the lift coefficient and aspect ratio of the wing.
    pub fn lift_induced_angle_of_attach(&self, lift_coefficient: f64) -> f64 {
        if self.aspect_ratio <= 0.0 {
            return 0.0;
        }

        lift_coefficient / (PI * self.aspect_ratio)
    }
}