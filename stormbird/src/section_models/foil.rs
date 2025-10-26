// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;
use stormath::{
    type_aliases::Float,
    consts::{PI, TAU},
    special_functions
};

use crate::error::Error;

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
    pub cl_zero_angle: Float,
    #[serde(default="Foil::default_cl_initial_slope")]
    /// How fast the lift coefficient increases with angle of attack, when the angle of attack is
    /// small. The default value is 2 * pi, which is a typical value for a normal foil profile,
    /// but it can also be set to different value for instance to account for boundary layer
    /// suction/blowing.
    pub cl_initial_slope: Float,
    #[serde(default)]
    /// Optional proportionality factor for adding higher order terms to the lift. Is zero by
    /// default, and therefore not used. Can be used to adjust the behavior of the lift curve close
    /// to stall.
    pub cl_high_order_factor: Float,
    #[serde(default)]
    /// Option power for adding higher order terms to the lift. Is zero by default, and therefore
    /// not used. Can be used to adjust the behavior of the lift curve close to stall.
    pub cl_high_order_power: Float,
    #[serde(default="Foil::default_one")]
    /// The maximum lift coefficient after stall.
    pub cl_max_after_stall: Float,
    #[serde(default)]
    /// Minimum drag coefficient when the angle of attack is equal to the `angle_cd_min`.
    pub cd_min: Float,
    #[serde(default)]
    /// The angle where the the minimum drag coefficient is reached
    pub angle_cd_min: Float,
    #[serde(default)]
    /// Factor to give the drag coefficient a second order term. This is zero by default.
    pub cd_second_order_factor: Float,
    #[serde(default="Foil::default_one")]
    /// The maximum drag coefficient after stall.
    pub cd_max_after_stall: Float,
    #[serde(default="Foil::default_cd_power_after_stall")]
    /// Power factor for the harmonic dependency of the drag coefficient after stall. Set to 1.6 by
    /// default.
    pub cd_power_after_stall: Float,
    #[serde(default)]
    /// factor that can be used to correct for numerical errors in the lift-induced drag. Set to a
    /// positive value to increase the drag, and a negative value to decrease the drag. The
    /// default is zero, which means no correction.
    pub cdi_correction_factor: Float,
    #[serde(default="Foil::default_mean_stall_angle")]
    /// The mean stall angle for positive angles of attack, which is the mean angle where the model transitions from pre-stall to
    /// post-stall behavior. The default value is 20 degrees.
    pub mean_positive_stall_angle: Float,
    #[serde(default="Foil::default_mean_stall_angle")]
    /// The mean stall angle for negative angles of attack, which is the mean angle where the model transitions from pre-stall to
    /// post-stall behavior. The default value is 20 degrees.
    pub mean_negative_stall_angle: Float,
    #[serde(default="Foil::default_stall_range")]
    /// The range of the stall transition. The default value is 6 degrees.
    pub stall_range: Float,
    #[serde(default)]
    /// Factor to model additional drag when the foil is stalling, but that is not included in the
    /// pre- and post-stall drag models.
    pub cd_bump_during_stall: Float,
    #[serde(default)]
    /// Optional offset to the stall angle for the drag coefficient. This can be used to tune the
    /// drag curve to better fit experimental data. The default is zero, which means that stall
    /// effects on the drag starts at the same angle of attack as for the lift. When the offset is
    /// set to any value, the "amount of stall" function is shifted by this value for the case of
    /// the drag coefficient only.
    pub cd_stall_angle_offset: Float,
    #[serde(default)]
    /// Factor to model added mass due to accelerating flow around the foil. Set to zero by default.
    pub added_mass_factor: Float,
}

fn get_stall_angle(angle_of_attack: Float) -> Float {
    let mut effective = angle_of_attack.abs();

        while effective > PI {
            effective -= PI;
        }

        effective *= angle_of_attack.signum();

    effective
}

impl Foil {
    fn default_one() -> Float {1.0}
    pub fn default_cl_initial_slope()     -> Float {TAU}
    pub fn default_mean_stall_angle()     -> Float {Float::from(20.0).to_radians()}
    pub fn default_stall_range()          -> Float {Float::from(6.0).to_radians()}
    pub fn default_cd_power_after_stall() -> Float {1.6}

    pub fn new_from_string(string: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(string)?;

        Ok(serde_res)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Calculates the lift coefficient for a given angle of attack.
    ///
    /// # Arguments
    /// * `angle_of_attack` - Angle of attack in radians.
    pub fn lift_coefficient(&self, angle_of_attack: Float) -> Float {
        let cl_pre_stall  = self.lift_coefficient_pre_stall_with_stall_drop_off(angle_of_attack);

        let cl_post_stall = self.lift_coefficient_post_stall_with_stall_weight(angle_of_attack);

        cl_post_stall + cl_pre_stall
    }

    #[inline(always)]
    pub fn lift_coefficient_linear(&self, angle_of_attack: Float) -> Float {
        self.cl_zero_angle + self.cl_initial_slope * angle_of_attack
    }

    #[inline(always)]
    pub fn lift_coefficient_pre_stall_raw(&self, angle_of_attack: Float) -> Float {
        let angle_high_power = if self.cl_high_order_power > 0.0 {
            angle_of_attack.abs().powf(self.cl_high_order_power) * angle_of_attack.signum()
        } else {
            0.0
        };

        self.lift_coefficient_linear(angle_of_attack) +
        self.cl_high_order_factor * angle_high_power
    }

    #[inline(always)]
    pub fn lift_coefficient_pre_stall_with_stall_drop_off(&self, angle_of_attack: Float) -> Float {
        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        let cl_pre_stall_raw = self.lift_coefficient_pre_stall_raw(angle_of_attack);

        cl_pre_stall_raw * (1.0 - amount_of_stall)
    }

    #[inline(always)]
    pub fn lift_coefficient_post_stall_raw(&self, angle_of_attack: Float) -> Float {
        let stall_angle = get_stall_angle(angle_of_attack);

        self.cl_max_after_stall * (2.0 * stall_angle).sin()
    }

    #[inline(always)]
    pub fn lift_coefficient_post_stall_with_stall_weight(&self, angle_of_attack: Float) -> Float {
        let cl_post_stall_raw = self.lift_coefficient_post_stall_raw(angle_of_attack);

        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        cl_post_stall_raw * amount_of_stall
    }

    /// Calculates the drag coefficient for a given angle of attack.
    ///
    /// # Arguments
    /// * `angle_of_attack` - Angle of attack in radians.
    pub fn drag_coefficient(&self, angle_of_attack: Float) -> Float {
        let stall_angle = get_stall_angle(angle_of_attack);

        let pre_stall_effective_angle = (angle_of_attack + self.angle_cd_min).abs();

        let cd_pre_stall  = self.cd_min + self.cd_second_order_factor * pre_stall_effective_angle.powi(2);
        let cd_post_stall = self.cd_max_after_stall * stall_angle.sin().abs().powf(self.cd_power_after_stall);

        let angle_for_stall_transition = angle_of_attack + self.cd_stall_angle_offset * angle_of_attack.signum();

        let cd_raw = self.combine_pre_and_post_stall(angle_for_stall_transition, cd_pre_stall, cd_post_stall);

        let cd_during_stall = self.additional_cd_during_stall(angle_for_stall_transition);

        if self.cdi_correction_factor != 0.0{
            let cl = self.lift_coefficient(angle_of_attack);

            let cdi_correction = self.cdi_correction_factor * cl.abs().powi(2);

            cd_raw + cdi_correction + cd_during_stall
        } else {
            cd_raw + cd_during_stall
        }
    }

    fn additional_cd_during_stall(&self, angle_of_attack: Float) -> Float {
        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        let stall_drag_factor = (1.0 - (amount_of_stall * TAU).cos()) * 0.5;

        self.cd_bump_during_stall * stall_drag_factor
    }

    /// Calculates the added mass force in the direction of the heave motion of the foil.
    ///
    /// # Arguments
    /// * `heave_acceleration` - Acceleration of the flow around the foil normal to the chord
    /// length. That is, the opposite of the acceleration of the foil itself.
    pub fn added_mass_coefficient(&self, heave_acceleration: Float) -> Float {
        self.added_mass_factor * heave_acceleration
    }

    /// Calculates the amount of stall for a given angle of attack. Varies between 0 and 1, where 0
    /// means no stall and 1 means full stall.
    pub fn amount_of_stall(&self, angle_of_attack: Float) -> Float {
        let effective_angle = angle_of_attack + self.angle_cd_min;

        let mean_stall_angle = if effective_angle >= 0.0 {
            self.mean_positive_stall_angle.abs()
        } else {
            self.mean_negative_stall_angle.abs()
        };

        special_functions::sigmoid_zero_to_one(
            effective_angle.abs(),
            mean_stall_angle,
            self.stall_range
        )
    }

    fn combine_pre_and_post_stall(&self, angle_of_attack: Float, pre_stall: Float, post_stall: Float) -> Float {
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
            cd_min:                 0.0,
            angle_cd_min:           0.0,
            cd_second_order_factor: 0.0,
            cd_max_after_stall:     Self::default_one(),
            cd_power_after_stall:   Self::default_cd_power_after_stall(),
            cdi_correction_factor:  0.0,
            mean_positive_stall_angle: Self::default_mean_stall_angle(),
            mean_negative_stall_angle: Self::default_mean_stall_angle(),
            stall_range:            Self::default_stall_range(),
            cd_stall_angle_offset:  0.0,
            cd_bump_during_stall:   0.0,
            added_mass_factor:      0.0
        }
    }
}
