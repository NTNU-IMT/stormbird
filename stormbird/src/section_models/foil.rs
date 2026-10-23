// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;
use stormath::{
    type_aliases::Float,
    consts::TAU,
    transition_functions
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
/// accurately as simple functions. The lift is mostly linear, but the slope can optionally be
/// reduced above a given angle, to model separation that develops gradually before the full
/// stall. The reduction is introduced smoothly using an integrated sigmoid function. The drag is assumed
/// to be represented as a second order polynomial, with the minimum value where the linear part of
/// the lift is zero.
/// 2) For angles of attack above stall, both the lift and drag are assumed to be harmonic functions
/// which primarily is adjusted by setting the *max value* after stall. This is a rough model, which
/// is assumed to be appropriate as the pre-stall behavior is usually more important for a
/// wind power device.
///
/// The transit between the two models is done using a sigmoid function, where both the transition
/// point and the width of the transition can be adjusted.
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
    /// Optional reduction of the lift slope for positive angles of attack above the onset angle.
    /// A value of 0.0 means no reduction, while 1.0 means that the slope is reduced to zero. Is
    /// zero by default, and therefore not used. Can be used to model separation that develops
    /// gradually before the full stall, for instance on a flap.
    pub cl_slope_reduction_factor_positive: Float,
    #[serde(default)]
    /// Same as `cl_slope_reduction_factor_positive`, but for negative angles of attack.
    pub cl_slope_reduction_factor_negative: Float,
    #[serde(default="Foil::default_cl_slope_reduction_onset_angle")]
    /// The angle of attack where the lift slope reduction is half-way developed, for positive
    /// angles of attack. The default value is 10 degrees.
    pub cl_slope_reduction_onset_angle_positive: Float,
    #[serde(default="Foil::default_cl_slope_reduction_onset_angle")]
    /// The angle of attack where the lift slope reduction is half-way developed, for negative
    /// angles of attack. Given as a positive value. The default value is 10 degrees.
    pub cl_slope_reduction_onset_angle_negative: Float,
    #[serde(default="Foil::default_stall_range")]
    /// The range of the transition to the reduced lift slope. Uses the same definition as the
    /// `stall_range`. The default value is 6 degrees.
    pub cl_slope_reduction_range: Float,
    #[serde(default="Foil::default_one")]
    /// The maximum lift coefficient after stall.
    pub cl_max_after_stall: Float,
    #[serde(default)]
    /// Minimum drag coefficient, which is assumed to be reached when the linear part of the lift
    /// is zero.
    pub cd_min: Float,
    #[serde(default)]
    /// Factor to give the drag coefficient a second order term. The term is proportional to the
    /// square of the linear part of the lift divided by the initial lift slope, which is
    /// approximately the square of the angle of attack relative to the zero-lift angle. This is
    /// zero by default.
    pub cd_second_order_factor: Float,
    #[serde(default="Foil::default_one")]
    /// The maximum drag coefficient after stall.
    pub cd_max_after_stall: Float,
    #[serde(default="Foil::default_cd_power_after_stall")]
    /// Power factor for the harmonic dependency of the drag coefficient after stall. Set to 1.6 by
    /// default.
    pub cd_power_after_stall: Float,
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
}

impl Foil {
    fn default_one() -> Float {1.0}
    pub fn default_cl_initial_slope()     -> Float {TAU}
    pub fn default_mean_stall_angle()     -> Float {Float::from(20.0_f32).to_radians()}
    pub fn default_stall_range()          -> Float {Float::from(6.0_f32).to_radians()}
    pub fn default_cl_slope_reduction_onset_angle() -> Float {Float::from(10.0_f32).to_radians()}
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
        self.cl_zero_angle + self.cl_initial_slope * angle_of_attack.sin()
    }

    /// Pre-stall lift coefficient, which is the linear lift with an optional reduction of the slope
    /// for both positive and negative angles of attack.
    ///
    /// The slope with respect to sin(angle_of_attack) is reduced by a sigmoid function, so that
    /// the reduction in the lift is the integral of the sigmoid function. Both the positive and
    /// negative reductions are always active, which makes the lift smooth for all angles.
    #[inline(always)]
    pub fn lift_coefficient_pre_stall_raw(&self, angle_of_attack: Float) -> Float {
        let x = angle_of_attack.sin();

        let x_onset_positive = self.cl_slope_reduction_onset_angle_positive.abs().sin();
        let x_onset_negative = self.cl_slope_reduction_onset_angle_negative.abs().sin();

        let reduction_positive = integrated_sigmoid_from_zero(
            x, x_onset_positive, self.cl_slope_reduction_range
        );

        let reduction_negative = integrated_sigmoid_from_zero(
            -x, x_onset_negative, self.cl_slope_reduction_range
        );

        self.lift_coefficient_linear(angle_of_attack) + self.cl_initial_slope * (
            -self.cl_slope_reduction_factor_positive * reduction_positive +
            self.cl_slope_reduction_factor_negative * reduction_negative
        )
    }

    #[inline(always)]
    pub fn lift_coefficient_pre_stall_with_stall_drop_off(&self, angle_of_attack: Float) -> Float {
        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        let cl_pre_stall_raw = self.lift_coefficient_pre_stall_raw(angle_of_attack);

        cl_pre_stall_raw * (1.0 - amount_of_stall)
    }

    #[inline(always)]
    pub fn lift_coefficient_post_stall_raw(&self, angle_of_attack: Float) -> Float {
        self.cl_max_after_stall * (2.0 * angle_of_attack).sin()
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
        let cd_pre_stall  = self.drag_coefficient_pre_stall(angle_of_attack);
        let cd_post_stall = self.cd_max_after_stall * angle_of_attack.sin().abs().powf(self.cd_power_after_stall);

        let amount_of_stall = self.amount_of_stall(angle_of_attack);

        cd_pre_stall * (1.0 - amount_of_stall) + amount_of_stall * cd_post_stall
    }

    /// Pre-stall drag coefficient, modeled as a second order polynomial with the minimum value
    /// at the zero-lift angle of the linear part of the lift.
    ///
    /// The linear lift is divided by the initial lift slope, so that the variable is approximately
    /// the angle of attack relative to the zero-lift angle. If the initial lift slope is zero, the
    /// zero-lift angle is undefined, and the sine of the angle of attack is used directly.
    #[inline(always)]
    pub fn drag_coefficient_pre_stall(&self, angle_of_attack: Float) -> Float {
        let effective_angle = if self.cl_initial_slope != 0.0 {
            self.lift_coefficient_linear(angle_of_attack) / self.cl_initial_slope
        } else {
            angle_of_attack.sin()
        };

        self.cd_min + self.cd_second_order_factor * effective_angle.powi(2)
    }

    /// Calculates the amount of stall for a given angle of attack. Varies between 0 and 1, where 0
    /// means no stall and 1 means full stall.
    pub fn amount_of_stall(&self, angle_of_attack: Float) -> Float {
        let mean_stall_angle = if angle_of_attack >= 0.0 {
            self.mean_positive_stall_angle.abs()
        } else {
            self.mean_negative_stall_angle.abs()
        };

        transition_functions::sigmoid_zero_to_one(
            angle_of_attack.abs(),
            mean_stall_angle,
            self.stall_range
        )
    }

    /// Returns the mirror image of the foil, i.e., the same foil profile with the suction side and
    /// pressure side swapped. For instance, a foil with a flap deflected to one side becomes a foil
    /// with the flap deflected to the other side.
    ///
    /// The mirrored foil satisfies C_L(alpha) = -C_L_original(-alpha) and
    /// C_D(alpha) = C_D_original(-alpha). This is achieved by changing the sign of the lift at zero
    /// angle of attack, and swapping all parameters that are given separately for positive and
    /// negative angles of attack. The other parameters are symmetric with respect to the angle of
    /// attack, and are kept as they are.
    pub fn mirrored(&self) -> Self {
        Self {
            cl_zero_angle: -self.cl_zero_angle,
            cl_initial_slope: self.cl_initial_slope,
            cl_slope_reduction_factor_positive: self.cl_slope_reduction_factor_negative,
            cl_slope_reduction_factor_negative: self.cl_slope_reduction_factor_positive,
            cl_slope_reduction_onset_angle_positive: self.cl_slope_reduction_onset_angle_negative,
            cl_slope_reduction_onset_angle_negative: self.cl_slope_reduction_onset_angle_positive,
            cl_slope_reduction_range: self.cl_slope_reduction_range,
            cl_max_after_stall: self.cl_max_after_stall,
            cd_min: self.cd_min,
            cd_second_order_factor: self.cd_second_order_factor,
            cd_max_after_stall: self.cd_max_after_stall,
            cd_power_after_stall: self.cd_power_after_stall,
            mean_positive_stall_angle: self.mean_negative_stall_angle,
            mean_negative_stall_angle: self.mean_positive_stall_angle,
            stall_range: self.stall_range,
        }
    }

    /// Linear interpolation between all the parameters of two foils, where a weight of 0.0 gives
    /// the first foil and a weight of 1.0 gives the second foil.
    pub fn interpolate(first: &Self, second: &Self, weight: Float) -> Self {
        let blend = |a: Float, b: Float| a + weight * (b - a);

        Self {
            cl_zero_angle: blend(first.cl_zero_angle, second.cl_zero_angle),
            cl_initial_slope: blend(first.cl_initial_slope, second.cl_initial_slope),
            cl_slope_reduction_factor_positive: blend(
                first.cl_slope_reduction_factor_positive, second.cl_slope_reduction_factor_positive
            ),
            cl_slope_reduction_factor_negative: blend(
                first.cl_slope_reduction_factor_negative, second.cl_slope_reduction_factor_negative
            ),
            cl_slope_reduction_onset_angle_positive: blend(
                first.cl_slope_reduction_onset_angle_positive, second.cl_slope_reduction_onset_angle_positive
            ),
            cl_slope_reduction_onset_angle_negative: blend(
                first.cl_slope_reduction_onset_angle_negative, second.cl_slope_reduction_onset_angle_negative
            ),
            cl_slope_reduction_range: blend(first.cl_slope_reduction_range, second.cl_slope_reduction_range),
            cl_max_after_stall: blend(first.cl_max_after_stall, second.cl_max_after_stall),
            cd_min: blend(first.cd_min, second.cd_min),
            cd_second_order_factor: blend(first.cd_second_order_factor, second.cd_second_order_factor),
            cd_max_after_stall: blend(first.cd_max_after_stall, second.cd_max_after_stall),
            cd_power_after_stall: blend(first.cd_power_after_stall, second.cd_power_after_stall),
            mean_positive_stall_angle: blend(first.mean_positive_stall_angle, second.mean_positive_stall_angle),
            mean_negative_stall_angle: blend(first.mean_negative_stall_angle, second.mean_negative_stall_angle),
            stall_range: blend(first.stall_range, second.stall_range),
        }
    }
}

impl Default for Foil {
    fn default() -> Self {
        Self {
            cl_zero_angle:          0.0,
            cl_initial_slope:       Self::default_cl_initial_slope(),
            cl_slope_reduction_factor_positive: 0.0,
            cl_slope_reduction_factor_negative: 0.0,
            cl_slope_reduction_onset_angle_positive: Self::default_cl_slope_reduction_onset_angle(),
            cl_slope_reduction_onset_angle_negative: Self::default_cl_slope_reduction_onset_angle(),
            cl_slope_reduction_range: Self::default_stall_range(),
            cl_max_after_stall:     Self::default_one(),
            cd_min:                 0.0,
            cd_second_order_factor: 0.0,
            cd_max_after_stall:     Self::default_one(),
            cd_power_after_stall:   Self::default_cd_power_after_stall(),
            mean_positive_stall_angle: Self::default_mean_stall_angle(),
            mean_negative_stall_angle: Self::default_mean_stall_angle(),
            stall_range:            Self::default_stall_range(),
        }
    }
}
