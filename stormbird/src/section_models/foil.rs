// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;
use math_utils::special_functions;

use std::f64::consts::PI;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum StallModel {
    #[default]
    Harmonic,
    ConstantLift,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Parametric model of a foil profile that can compute lift and drag coefficients.
/// 
/// The reason for using a parametric model, rather than a data based table look-up, is two fold:
/// 
/// 1) It becomes easier to use this model as a building block for more complex foil models, where
/// the behavior depends on some internal state, such as flap angle or suction rate, because the 
/// parameters can be allowed to depend on the internal state through interpolation.
/// 2) A parametric model ensures smoothness, which is important when using the model in 
/// some form optimization, for instance to maximize thrust for a given wind direction. The 
/// smoothness is in particular practical when the expected optimal point is close to the stall 
/// angle 
/// 
/// The model is divided in two core sub-models
/// 1) For angles of attack below stall, it is assumed that both lift and drag can be represented 
/// accurately as simple polynomials. The lift is mostly linear, but can also have an optional 
/// high-order term where both the factor and power of the term is adjustable. The drag is assumed 
/// to be represented as a second order polynomial.
/// 2) FOr angles of attack above stall, both the lift and drag are assumed to be harmonic functions
/// which primarily is adjusted by setting the *max value* after stall. This is a rough model, which 
/// is assumed to be appropriate as the pre-stall behavior is usually more important for a 
/// wind power device.
/// 
/// The transit between the two models is done using a sigmoid function, where both the transition
/// point and the width of the transition can be adjusted.
/// 
/// In addition, there factors in the model to account for added mass and lift due to the time 
/// derivative of the angle of attack. Both these effects are assumed to be linear for simplicity.
pub struct Foil {
    #[serde(default)]
    /// Lift coefficient at zero angle of attack. This is zero by default, but can be set to a 
    /// non-zero value to account for camber, flap angle or boundary layer suction/blowing.
    pub cl_zero_angle: f64,
    #[serde(default="Foil::default_cl_initial_slope")]
    /// How fast the lift coefficient increases with angle of attack, when the angle of attack is
    /// small. The default value is 2 * pi, which is a typical value for a normal foil profile, 
    /// but it can also be set to different value for instance to account for boundary layer 
    /// suction/blowing.
    pub cl_initial_slope: f64,
    #[serde(default)]
    /// Optional proportionality factor for adding higher order terms to the lift. Is zero by 
    /// default, and therefore not used. Can be used to adjust the behavior of the lift curve close
    /// to stall.
    pub cl_high_order_factor: f64,
    #[serde(default)]
    /// Option power for adding higher order terms to the lift. Is zero by default, and therefore 
    /// not used. Can be used to adjust the behavior of the lift curve close to stall.
    pub cl_high_order_power: f64,
    #[serde(default="Foil::default_one")]
    /// The maximum lift coefficient after stall. 
    pub cl_max_after_stall: f64,
    #[serde(default)]
    /// Drag coefficient at zero angle of attack
    pub cd_zero_angle: f64,
    #[serde(default)]
    /// Factor to give the drag coefficient a second order term. This is zero by default.
    pub cd_second_order_factor: f64,
    #[serde(default="Foil::default_one")]
    /// The maximum drag coefficient after stall.
    pub cd_max_after_stall: f64,
    #[serde(default="Foil::default_cd_power_after_stall")]
    /// Power factor for the harmonic dependency of the drag coefficient after stall. Set to 1.6 by 
    /// default.
    pub cd_power_after_stall: f64,
    #[serde(default="Foil::default_mean_stall_angle")]
    /// The mean stall angle for positive angles of attack, which is the mean angle where the model transitions from pre-stall to
    /// post-stall behavior. The default value is 20 degrees.
    pub mean_positive_stall_angle: f64,
    #[serde(default="Foil::default_mean_stall_angle")]
    /// The mean stall angle for negative angles of attack, which is the mean angle where the model transitions from pre-stall to
    /// post-stall behavior. The default value is 20 degrees.
    pub mean_negative_stall_angle: f64,
    #[serde(default="Foil::default_stall_range")]
    /// The range of the stall transition. The default value is 6 degrees.
    pub stall_range: f64,
    #[serde(default)]
    /// Factor to model lift due to changing angle of attack. This is zero by default, and therefore
    /// not used.
    pub cl_changing_aoa_factor: f64,
    #[serde(default)]
    /// Factor to model added mass due to accelerating flow around the foil. Set to zero by default.
    pub added_mass_factor: f64,
    #[serde(default)]
    /// Type of stall model to use. The default is harmonic.
    pub stall_model: StallModel,
}

fn get_stall_angle(angle_of_attack: f64) -> f64 {
    let mut effective = angle_of_attack.abs();

        while effective > PI {
            effective -= PI;
        }

        effective *= angle_of_attack.signum();
    
    effective
}

impl Foil {
    fn default_one() -> f64 {1.0}
    pub fn default_cl_initial_slope()     -> f64 {2.0 * PI}
    pub fn default_mean_stall_angle()     -> f64 {20.0_f64.to_radians()}
    pub fn default_stall_range()          -> f64 {6.0_f64.to_radians()}
    pub fn default_cd_power_after_stall() -> f64 {1.6}

    pub fn new_from_string(string: &str) -> Self {
        serde_json::from_str(string).unwrap()
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    
    /// Calculates the lift coefficient for a given angle of attack.
    /// 
    /// # Arguments
    /// * `angle_of_attack` - Angle of attack in radians.
    pub fn lift_coefficient(&self, angle_of_attack: f64) -> f64 {
        let cl_pre_stall  = self.lift_coefficient_pre_stall(angle_of_attack);

        match self.stall_model {
            StallModel::Harmonic => {
                let stall_angle = get_stall_angle(angle_of_attack);

                let cl_post_stall = self.lift_coefficient_post_stall(stall_angle);

                self.combine_pre_and_post_stall(angle_of_attack, cl_pre_stall, cl_post_stall)
            },
            StallModel::ConstantLift => {
                let mean_stall_angle = if angle_of_attack >= 0.0 {
                    self.mean_positive_stall_angle.abs()
                } else {
                    self.mean_negative_stall_angle.abs()
                };

                let cl_post_stall = self.lift_coefficient_pre_stall(mean_stall_angle * angle_of_attack.signum());

                cl_pre_stall.abs().min(cl_post_stall.abs())*cl_pre_stall.signum()
            }
        }
    }

    pub fn lift_coefficient_pre_stall(&self, angle_of_attack: f64) -> f64 {
        let angle_high_power = if self.cl_high_order_power > 0.0 {
            angle_of_attack.abs().powf(self.cl_high_order_power) * angle_of_attack.signum()
        } else {
            0.0
        };
        
        self.cl_zero_angle + 
        self.cl_initial_slope * angle_of_attack +
        self.cl_high_order_factor * angle_high_power
    }

    pub fn lift_coefficient_post_stall(&self, angle_of_attack: f64) -> f64 {
        self.cl_max_after_stall * (2.0 * angle_of_attack).sin()
    }

    /// Calculates the drag coefficient for a given angle of attack.
    /// 
    /// # Arguments
    /// * `angle_of_attack` - Angle of attack in radians.
    pub fn drag_coefficient(&self, angle_of_attack: f64) -> f64 {
        let stall_angle = get_stall_angle(angle_of_attack);

        let cd_pre_stall  = self.cd_zero_angle + self.cd_second_order_factor * angle_of_attack.powi(2);
        let cd_post_stall = self.cd_max_after_stall * stall_angle.sin().abs().powf(self.cd_power_after_stall);

        self.combine_pre_and_post_stall(angle_of_attack, cd_pre_stall, cd_post_stall)
    }

    /// Calculates the added mass force in the direction of the heave motion of the foil.
    /// 
    /// # Arguments
    /// * `heave_acceleration` - Acceleration of the flow around the foil normal to the chord 
    /// length. That is, the opposite of theacceleration of the foil itself.
    pub fn added_mass_coefficient(&self, heave_acceleration: f64) -> f64 {
        self.added_mass_factor * heave_acceleration
    }

    /// Calculates the amount of stall for a given angle of attack.
    pub fn amount_of_stall(&self, angle_of_attack: f64) -> f64 {
        let mean_stall_angle = if angle_of_attack >= 0.0 {
            self.mean_positive_stall_angle.abs()
        } else {
            self.mean_negative_stall_angle.abs()
        };

        special_functions::sigmoid_zero_to_one(
            angle_of_attack.abs(), 
            mean_stall_angle, 
            self.stall_range
        )
    }

    fn combine_pre_and_post_stall(&self, angle_of_attack: f64, pre_stall: f64, post_stall: f64) -> f64 {
        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        pre_stall * (1.0 - amount_of_stall) + amount_of_stall * post_stall
    }
}

impl Default for Foil {
    fn default() -> Self {
        Self {
            cl_zero_angle:          0.0,
            cl_initial_slope:       Self::default_cl_initial_slope(),
            cl_high_order_factor:   0.0,
            cl_high_order_power:    0.0,
            cl_max_after_stall:     Self::default_one(),
            cd_zero_angle:          0.0,
            cd_second_order_factor: 0.0,
            cd_max_after_stall:     Self::default_one(),
            cd_power_after_stall:   Self::default_cd_power_after_stall(),
            mean_positive_stall_angle: Self::default_mean_stall_angle(),
            mean_negative_stall_angle: Self::default_mean_stall_angle(),
            stall_range:            Self::default_stall_range(),
            cl_changing_aoa_factor: 0.0,
            added_mass_factor:      0.0,
            stall_model:            StallModel::Harmonic,
        }
    }
}