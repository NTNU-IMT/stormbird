// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use super::*;

use crate::error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "VaryingFoilInput")]
/// A foil profile where the parameters can vary depending on an internal state.
///
/// The two typical use cases are to model foil sections that include a flap angle, or suction
/// sails, where the foil section properties are dependent on the suction rate.
///
/// The parameters of the foil are linearly interpolated between the foils in the data, and are
/// constant outside the range of the data.
///
/// If the internal state data only contains values with one sign, i.e., only positive or only
/// negative values, the foil is assumed to be symmetric with respect to the sign of the internal
/// state. An internal state with the opposite sign then gives the mirrored foil at the absolute
/// value of the internal state. This means that, for instance, the data for a flap angle only
/// needs to be given for positive flap angles. If the data contains both positive and negative
/// values, the data is used directly, without any mirroring.
pub struct VaryingFoil {
    /// The internal state values for the foils in the data, in increasing order
    pub internal_state_data: Vec<Float>,
    /// The foil models, one for each internal state value
    pub foils_data: Vec<Foil>,
}

#[derive(Deserialize)]
/// Unchecked version of the varying foil, used for deserialization
struct VaryingFoilInput {
    internal_state_data: Vec<Float>,
    foils_data: Vec<Foil>,
}

impl TryFrom<VaryingFoilInput> for VaryingFoil {
    type Error = String;

    fn try_from(input: VaryingFoilInput) -> Result<Self, Self::Error> {
        Self::new(input.internal_state_data, input.foils_data)
    }
}

impl VaryingFoil {
    /// Creates a new varying foil, and checks that the data is valid
    pub fn new(internal_state_data: Vec<Float>, foils_data: Vec<Foil>) -> Result<Self, String> {
        if internal_state_data.is_empty() {
            return Err("The varying foil must have at least one foil in the data".to_string());
        }

        if internal_state_data.len() != foils_data.len() {
            return Err(format!(
                "The internal state data and the foils data must have the same length. Internal \
                state data: {}, foils data: {}",
                internal_state_data.len(), foils_data.len()
            ));
        }

        for i in 1..internal_state_data.len() {
            if internal_state_data[i] <= internal_state_data[i - 1] {
                return Err(
                    "The internal state data for the varying foil must be strictly increasing"
                    .to_string()
                );
            }
        }

        Ok(Self { internal_state_data, foils_data })
    }

    pub fn new_from_string(input_str: &str) -> Result<Self, Error> {
        let serde_res = serde_json::from_str(input_str)?;

        Ok(serde_res)
    }

    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    /// Returns the sign of the internal state data if all values have the same sign, i.e., if the
    /// data only covers one side of the internal state. Returns `None` if the data contains both
    /// positive and negative values. Zero counts as positive, unless all values are zero or
    /// negative.
    pub fn one_sided_data_sign(&self) -> Option<Float> {
        let first = self.internal_state_data[0];
        let last = self.internal_state_data[self.internal_state_data.len() - 1];

        if first >= 0.0 {
            Some(1.0)
        } else if last <= 0.0 {
            Some(-1.0)
        } else {
            None
        }
    }

    /// Returns the foil for a given internal state.
    ///
    /// If the data is one-sided, and the internal state has the opposite sign of the data, the
    /// mirrored foil at the absolute value of the internal state is returned.
    pub fn get_foil(&self, internal_state: Float) -> Foil {
        match self.one_sided_data_sign() {
            Some(data_sign) if internal_state * data_sign < 0.0 => {
                self.interpolated_foil(-internal_state).mirrored()
            },
            _ => self.interpolated_foil(internal_state),
        }
    }

    /// Linear interpolation of the foil parameters directly on the data, without any mirroring.
    /// The first or last foil is used if the internal state is outside the range of the data.
    fn interpolated_foil(&self, internal_state: Float) -> Foil {
        let x_data = &self.internal_state_data;
        let n = x_data.len();

        if internal_state <= x_data[0] {
            return self.foils_data[0].clone();
        }

        if internal_state >= x_data[n - 1] {
            return self.foils_data[n - 1].clone();
        }

        // Index of the last data point that is less than or equal to the internal state. Always
        // in the range [0, n - 2] at this point, due to the checks above.
        let index = x_data.partition_point(|&x| x <= internal_state) - 1;

        let weight = (internal_state - x_data[index]) / (x_data[index + 1] - x_data[index]);

        Foil::interpolate(&self.foils_data[index], &self.foils_data[index + 1], weight)
    }

    pub fn lift_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient(angle_of_attack)
    }

    pub fn lift_coefficient_linear(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_linear(angle_of_attack)
    }

    pub fn lift_coefficient_pre_stall_raw(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_pre_stall_raw(angle_of_attack)
    }

    pub fn lift_coefficient_pre_stall_with_stall_drop_off(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_pre_stall_with_stall_drop_off(angle_of_attack)
    }

    pub fn lift_coefficient_post_stall_raw(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_post_stall_raw(angle_of_attack)
    }

    pub fn lift_coefficient_post_stall_with_stall_weight(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).lift_coefficient_post_stall_with_stall_weight(angle_of_attack)
    }

    pub fn lift_coefficient_derivative(&self, internal_state: Float) -> Float {
        self.get_foil(internal_state).cl_initial_slope
    }

    pub fn drag_coefficient(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).drag_coefficient(angle_of_attack)
    }


    pub fn amount_of_stall(&self, angle_of_attack: Float, internal_state: Float) -> Float {
        self.get_foil(internal_state).amount_of_stall(angle_of_attack)
    }
}
