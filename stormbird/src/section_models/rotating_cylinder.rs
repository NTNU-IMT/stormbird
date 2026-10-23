// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::consts::PI;

use super::*;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model representing a rotating cylinder. The lift, drag and power can be calculated
/// based on how fast the cylinder is spinning.
///
/// The lift has an initial slope with respect to the spin ratio, which changes smoothly to a
/// different slope at high spin ratios. The transition is the integral of a sigmoid function, and
/// is applied symmetrically, so that the lift is an odd function of the spin ratio: the sign of the
/// spin ratio only determines the sign of the lift.
pub struct RotatingCylinder {
    #[serde(default = "RotatingCylinder::default_cl_initial_slope")]
    /// How fast the lift coefficient increases with the spin ratio, when the spin ratio is small.
    pub cl_initial_slope: Float,
    #[serde(default = "RotatingCylinder::default_cl_high_spin_ratio_slope")]
    /// How fast the lift coefficient increases with the spin ratio, when the spin ratio is well
    /// above the transition spin ratio.
    pub cl_high_spin_ratio_slope: Float,
    #[serde(default = "RotatingCylinder::default_cl_slope_transition_spin_ratio")]
    /// The absolute value of the spin ratio where the transition from the initial slope to the high
    /// spin ratio slope is half-way developed.
    pub cl_slope_transition_spin_ratio: Float,
    #[serde(default = "RotatingCylinder::default_cl_slope_transition_range")]
    /// The range of the transition between the two slopes, with the same definition as the
    /// `stall_range` in the foil model. That is, the transition is 1% developed at the transition
    /// spin ratio minus the range, and 99% developed at the transition spin ratio plus the range.
    pub cl_slope_transition_range: Float,

    #[serde(default = "RotatingCylinder::default_cd_start")]
    /// Drag at spin ratio = 0.0
    pub cd_start: Float,
    #[serde(default = "RotatingCylinder::default_cd_end")]
    /// Drag at larger spin ratios
    pub cd_end: Float,
    #[serde(default = "RotatingCylinder::default_cd_angle_start")]
    /// Angle in radians that defines the slope of the drag curve at the beginning (higher value -> 
    /// steeper downwards slope)
    pub cd_angle_start: Float,
    #[serde(default = "RotatingCylinder::default_cd_spin_ratio_low_drag")]
    /// The spin ratio where the low drag is used
    pub cd_spin_ratio_low_drag: Float,
    #[serde(default = "RotatingCylinder::default_power_coefficient_polynomial")]
    /// Coefficients of a polynomial in the absolute spin ratio, used to calculate the power 
    /// coefficient, c_P = P / (0.5 * density * A * U^3), where A is the projected area. Given in 
    /// increasing order, i.e., c_P = sum_i (coefficient_i * |SR|^i). 
    pub power_coefficient_polynomial: Vec<Float>,
}

impl Default for RotatingCylinder {
    fn default() -> Self {
        RotatingCylinder {
            cl_initial_slope: Self::default_cl_initial_slope(),
            cl_high_spin_ratio_slope: Self::default_cl_high_spin_ratio_slope(),
            cl_slope_transition_spin_ratio: Self::default_cl_slope_transition_spin_ratio(),
            cl_slope_transition_range: Self::default_cl_slope_transition_range(),
            cd_start: Self::default_cd_start(),
            cd_end: Self::default_cd_end(),
            cd_angle_start: Self::default_cd_angle_start(),
            cd_spin_ratio_low_drag: Self::default_cd_spin_ratio_low_drag(),
            power_coefficient_polynomial: Self::default_power_coefficient_polynomial(),
        }
    }
}

impl std::fmt::Display for RotatingCylinder {
    /// Formats the model as a JSON string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;

        write!(f, "{}", json)
    }
}

impl RotatingCylinder {
    // The default lift parameters are fitted to two dimensional CFD simulations, from:
    // Ostman et al. (2023). Calculation of Flettner rotor forces using lifting line and CFD
    // methods. NuTTS conference.
    /// Default initial lift slope
    pub fn default_cl_initial_slope() -> Float {3.0}
    /// Default lift slope at high spin ratios
    pub fn default_cl_high_spin_ratio_slope() -> Float {1.5}
    /// Default spin ratio for the transition between the lift slopes
    pub fn default_cl_slope_transition_spin_ratio() -> Float {3.4}
    /// Default range of the transition between the lift slopes
    pub fn default_cl_slope_transition_range() -> Float {1.0}

    /// Default drag at zero spin ratio
    pub fn default_cd_start() -> Float {0.457}
    /// Default drag at large spin ratios
    pub fn default_cd_end() -> Float {0.05}
    /// Default angle that defines the initial slope of the drag curve (5 degrees)
    pub fn default_cd_angle_start() -> Float {Float::from(5.0_f32).to_radians()}
    /// Default spin ratio where the low drag is used
    pub fn default_cd_spin_ratio_low_drag() -> Float {2.7}

    /// Default power coefficient polynomial, from equation (9) in:
    /// Tillig, F., Ringsberg, J. W. (2020). Design, operation and analysis of wind-assisted cargo 
    /// ships. Ocean Engineering, 211, 107603. <https://doi.org/10.1016/j.oceaneng.2020.107603>
    /// 
    /// The polynomial is valid for rotors with an aspect ratio of 6 and an end disk diameter of two 
    /// times the rotor diameter.
    pub fn default_power_coefficient_polynomial() -> Vec<Float> {
        vec![0.0, 0.0234, -0.0168, 0.0143, -0.0004, 0.0001]
    }
    
    /// Creates a rotating cylinder from a JSON string
    pub fn new_from_string(input_string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(input_string)?;

        Ok(serde_res)
    }


    /// Calculates the lift coefficient for a given spin ratio.
    ///
    /// The slope of the lift with respect to the spin ratio changes from the initial slope to the
    /// high spin ratio slope through sigmoid functions centered at plus and minus the transition
    /// spin ratio. The lift is the integral of the slope from zero spin ratio, which makes it an odd
    /// and smooth function of the spin ratio.
    pub fn lift_coefficient(&self, spin_ratio: Float) -> Float {
        let transition_spin_ratio = self.cl_slope_transition_spin_ratio.abs();

        let transition = integrated_sigmoid_from_zero(
            spin_ratio, transition_spin_ratio, self.cl_slope_transition_range
        ) - integrated_sigmoid_from_zero(
            -spin_ratio, transition_spin_ratio, self.cl_slope_transition_range
        );

        self.cl_initial_slope * spin_ratio -
            (self.cl_initial_slope - self.cl_high_spin_ratio_slope) * transition
    }

    /// Calculates the drag coefficient for a given spin ratio
    pub fn drag_coefficient(&self, spin_ratio: Float) -> Float {   
        let angle0 = PI * spin_ratio.abs() / self.cd_spin_ratio_low_drag;
        let angle = angle0 + self.cd_angle_start;

        let angle_effective = PI * angle / (PI + self.cd_angle_start);

        if angle_effective < PI {
            let c1 = (1.0 + angle_effective.cos())/2.0;
            let c0 = (1.0 + self.cd_angle_start.cos())/2.0;

            let s = c1/c0;

            self.cd_start * s + (1.0 - s) * self.cd_end
        } else {
            self.cd_end
        }
    }

    /// Calculates the power coefficient from the polynomial model, using the absolute value of the
    /// spin ratio, as the power does not depend on the direction of rotation.
    pub fn power_coefficient(&self, spin_ratio: Float) -> Float {
        let spin_ratio_abs = spin_ratio.abs();

        // Horner's method
        self.power_coefficient_polynomial.iter().rev().fold(0.0, |result, coefficient| {
            result * spin_ratio_abs + coefficient
        })
    }


    /// Helper function to calculate revolutions per second from a target spin ratio, diameter and
    /// velocity.
    pub fn revolutions_per_second_from_spin_ratio(
        spin_ratio: Float, 
        diameter: Float, 
        velocity: Float
    ) -> Float {
        if velocity == 0.0 {
            0.0
        } else {
            let circumference = PI * diameter;
            let tangential_velocity = velocity * spin_ratio;
            
            tangential_velocity / circumference
        }
    }

    /// Calculates non-dimensional spin ratio, defined as the ratio of the surface velocity of the 
    /// cylinder to the free stream velocity.
    pub fn spin_ratio_from_revolutions_per_second(
        revolutions_per_second: Float,
        diameter: Float, 
        velocity: Float
    ) -> Float {
        if velocity == 0.0 {
            0.0
        } else {
            let circumference = PI * diameter;
            let tangential_velocity = circumference * revolutions_per_second;
    
            -tangential_velocity / velocity
        }   
    }
}
