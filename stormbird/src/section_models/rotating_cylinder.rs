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
    #[serde(default = "RotatingCylinder::default_cd_data")]
    /// Drag coefficient data as a function of spin ratio
    pub cd_data: Vec<Float>,
    #[serde(default)]
    /// Optional specification of non-zero angles of the wake behind the cylinder, as a function of spin ratio.
    pub wake_angle_data: Option<Vec<Float>>,
    #[serde(default)]
    /// Added mass factor for the cylinder
    pub added_mass_factor: Float,
    #[serde(default)]
    /// Two-dimensional moment of inertia
    pub moment_of_inertia_2d: Float,
}

impl Default for RotatingCylinder {
    fn default() -> Self {
        RotatingCylinder {
            revolutions_per_second: 0.0,
            spin_ratio_data: Self::default_spin_ratio_data(),
            cl_data: Self::default_cl_data(),
            cd_data: Self::default_cd_data(),
            wake_angle_data: None,
            added_mass_factor: 0.0,
            moment_of_inertia_2d: 0.0,
        }
    }
}

impl RotatingCylinder {
    pub fn new_from_string(input_string: &str) -> Self {
        serde_json::from_str(input_string).unwrap()
    }

    /// Default values for spin ratio data based on two dimensional CFD simulations
    pub fn default_spin_ratio_data() -> Vec<Float> {
        vec![0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0]
    }

    /// Default values for cl data based on two dimensional CFD simulations
    pub fn default_cl_data() -> Vec<Float> {
        vec![0.0, 1.22, 2.56, 5.93, 9.10, 10.77, 12.80, 13.71, 16.90]
    }

    /// Default values for cd data based on two dimensional CFD simulations
    pub fn default_cd_data() -> Vec<Float> {
        vec![0.457, 0.411, 0.296, 0.093, 0.066, 0.042, 0.064, 0.05, 0.076]
    }

    /// Calculates non-dimensional spin ratio, defined as the ratio of the surface velocity of the 
    /// cylinder to the free stream velocity.
    pub fn spin_ratio(&self, diameter: Float, velocity: Float) -> Float {
        let circumference = PI * diameter;
        let tangential_velocity = circumference * self.revolutions_per_second;

        -tangential_velocity / velocity
    }

    pub fn lift_coefficient_from_spin_ratio(&self, spin_ratio: Float) -> Float {
        let cl = interpolation::linear_interpolation(
            spin_ratio.abs(), 
            &self.spin_ratio_data, 
            &self.cl_data
        );

        cl * spin_ratio.signum()
    }

    pub fn drag_coefficient_from_spin_ratio(&self, spin_ratio: Float) -> Float {
        interpolation::linear_interpolation(
            spin_ratio.abs(), 
            &self.spin_ratio_data, 
            &self.cd_data,
        )
    }

    pub fn lift_coefficient(&self, diameter: Float, velocity: Float) -> Float {
        let spin_ratio = self.spin_ratio(diameter, velocity);

        self.lift_coefficient_from_spin_ratio(spin_ratio)
    }

    pub fn drag_coefficient(&self, diameter: Float, velocity: Float) -> Float {
        let spin_ratio = self.spin_ratio(diameter, velocity);

        self.drag_coefficient_from_spin_ratio(spin_ratio)
    }

    pub fn wake_angle(&self, diameter: Float, velocity: Float) -> Float {
        if let Some(wake_angle_data) = &self.wake_angle_data {
            let spin_ratio = self.spin_ratio(diameter, velocity);

            let angle_magnitude = interpolation::linear_interpolation(spin_ratio.abs(), &self.spin_ratio_data, &wake_angle_data);

            -angle_magnitude * spin_ratio.signum()
        } else {
            0.0
        }
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