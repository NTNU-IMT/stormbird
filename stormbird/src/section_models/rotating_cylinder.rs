// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use stormath::consts::PI;

use super::*;

use stormath::interpolation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model representing a rotating cylinder. The lift, drag and moment can be calculated based on how 
/// fast the cylinder is spinning. 
pub struct RotatingCylinder {
    #[serde(default)]
    /// The rotational speed of the rotor, in revolutions per second.
    pub revolutions_per_second: Float,
    #[serde(default = "RotatingCylinder::default_spin_ratio_data")]
    
    /// Spin ratio data used when interpolating lift and drag coefficients.
    pub spin_ratio_data: Vec<Float>,
    #[serde(default = "RotatingCylinder::default_cl_data")]
    /// Lift coefficient data as a function of spin ratio
    pub cl_data: Vec<Float>,
    
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
    
    
    #[serde(default)]
    /// Added mass factor for the cylinder
    pub added_mass_factor: Float,
    #[serde(default)]
    /// Two-dimensional moment of inertia
    pub moment_of_inertia_2d: Float,
    #[serde(default)]
    /// factor that can be used to correct for numerical errors in the lift-induced drag. Set to a
    /// positive value to increase the drag, and a negative value to decrease the drag. The
    /// default is zero, which means no correction.
    pub cdi_correction_factor: Float,
}

impl Default for RotatingCylinder {
    fn default() -> Self {
        RotatingCylinder {
            revolutions_per_second: 0.0,
            spin_ratio_data: Self::default_spin_ratio_data(),
            cl_data: Self::default_cl_data(),
            cd_start: Self::default_cd_start(),
            cd_end: Self::default_cd_end(),
            cd_angle_start: Self::default_cd_angle_start(),
            cd_spin_ratio_low_drag: Self::default_cd_spin_ratio_low_drag(),
            added_mass_factor: 0.0,
            moment_of_inertia_2d: 0.0,
            cdi_correction_factor: 0.0
        }
    }
}

impl RotatingCylinder {
    /// Default values for spin ratio data based on two dimensional CFD simulations
    pub fn default_spin_ratio_data() -> Vec<Float> {
        vec![0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0]
    }

    /// Default values for cl data based on two dimensional CFD simulations
    pub fn default_cl_data() -> Vec<Float> {
        vec![0.0, 1.22, 2.56, 5.93, 9.10, 10.77, 12.80, 13.71, 16.90]
    }

    pub fn default_cd_start() -> Float {0.457}
    pub fn default_cd_end() -> Float {0.05}
    pub fn default_cd_angle_start() -> Float {Float::from(5.0_f32).to_radians()}
    pub fn default_cd_spin_ratio_low_drag() -> Float {2.7}
    
    pub fn new_from_string(input_string: &str) -> Self {
        serde_json::from_str(input_string).unwrap()
    }

    /// Calculates non-dimensional spin ratio, defined as the ratio of the surface velocity of the 
    /// cylinder to the free stream velocity.
    pub fn spin_ratio(&self, diameter: Float, velocity: Float) -> Float {
        let circumference = PI * diameter;
        let tangential_velocity = circumference * self.revolutions_per_second;

        -tangential_velocity / velocity
    }

    pub fn lift_coefficient_from_spin_ratio(&self, spin_ratio: Float) -> Float {
        let len_data = self.spin_ratio_data.len();

        let spin_ratio_abs = spin_ratio.abs();

        let cl = if spin_ratio_abs > self.spin_ratio_data[len_data-1] {
            let delta_s = self.spin_ratio_data[len_data-1] - self.spin_ratio_data[len_data-2];
            let delta_cl = self.cl_data[len_data-1] - self.cl_data[len_data-2];

            let extrapolate = (
                spin_ratio_abs - 
                self.spin_ratio_data[len_data-1]
            ) * delta_cl / delta_s;

            self.cl_data[len_data-1] + extrapolate
        } else {
            interpolation::linear_interpolation(
                spin_ratio_abs,
                &self.spin_ratio_data, 
                &self.cl_data
            )
        };

        cl * spin_ratio.signum()
    }

    pub fn drag_coefficient_from_spin_ratio(&self, spin_ratio: Float) -> Float {   
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

    pub fn lift_coefficient(&self, diameter: Float, velocity: Float) -> Float {
        let spin_ratio = self.spin_ratio(diameter, velocity);

        self.lift_coefficient_from_spin_ratio(spin_ratio)
    }

    pub fn drag_coefficient(&self, diameter: Float, velocity: Float) -> Float {
        let spin_ratio = self.spin_ratio(diameter, velocity);

        let mut cd = self.drag_coefficient_from_spin_ratio(spin_ratio);

        if self.cdi_correction_factor != 0.0{
            let cl = self.lift_coefficient_from_spin_ratio(spin_ratio);

            let cdi_correction = self.cdi_correction_factor * cl.abs().powi(2);

            cd += cdi_correction;
        } 

        cd
    }

    pub fn added_mass_coefficient(&self, acceleration_magnitude: Float) -> Float {
        self.added_mass_factor * acceleration_magnitude
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
}