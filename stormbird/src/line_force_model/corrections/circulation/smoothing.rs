
use serde::{Serialize, Deserialize};

use stormath::smoothing::{
    end_condition::EndCondition, 
    gaussian::GaussianSmoothing, 
    polynomial::{
        CubicPolynomialSmoothing,
        WindowSize
    }
};

use crate::line_force_model::LineForceModel;
use super::prescribed::PrescribedCirculation;

#[derive(Debug, Clone)]
/// Enum for choosing the type of smoothing to apply to the circulation strength.
pub enum SmoothingType {
    /// Gaussian smoothing. The settings are stored in a vector, one for each wing in the model.
    Gaussian(Vec<GaussianSmoothing<f64>>),
    /// Cubic polynomial smoothing. The settings are stored in a vector, one for each wing in the 
    /// model.
    CubicPolynomial(Vec<CubicPolynomialSmoothing<f64>>),
}

#[derive(Debug, Clone)]
/// Struct that holds the settings for circulation smoothing.
pub struct CirculationSmoothing {
    /// The type of smoothing to apply.
    pub smoothing_type: SmoothingType,
    /// Optional prescribed circulation to subtract before smoothing, so that the smoothing is only
    /// applied to the residual circulation. This is useful for avoiding too large smoothing of  
    /// the end shape of the circulation distribution.
    pub prescribed_to_subtract_before_smoothing: Option<PrescribedCirculation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GaussianSmoothingBuilder {
    pub smoothing_length_factor: f64,
    #[serde(default)]
    pub number_of_end_points_to_interpolate: usize
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CubicPolynomialSmoothingBuilder {
    pub window_size: WindowSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SmoothingTypeBuilder {
    Gaussian(GaussianSmoothingBuilder),
    CubicPolynomial(CubicPolynomialSmoothingBuilder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CirculationSmoothingBuilder {
    pub smoothing_type: SmoothingTypeBuilder,
    #[serde(default)]
    pub prescribed_to_subtract_before_smoothing: Option<PrescribedCirculation>,
}

impl CirculationSmoothingBuilder {
    pub fn end_conditions(&self, line_force_model: &LineForceModel) -> Vec<[EndCondition<f64>; 2]> {
        let nr_wings = line_force_model.nr_wings();

        let mut end_conditions: Vec<[EndCondition<f64>; 2]> = Vec::new();

        for wing_index in 0..nr_wings {
            let non_zero_circulation_at_ends = line_force_model.non_zero_circulation_at_ends[wing_index];

            let mut end_conditions_current: [EndCondition<f64>; 2] = [
                EndCondition::Extended, 
                EndCondition::Extended
            ];

            for i in 0..2 {
                end_conditions_current[i] = if non_zero_circulation_at_ends[i] {
                    EndCondition::Extended
                } else {
                    EndCondition::Zero
                };
            }

            end_conditions.push(end_conditions_current);
        }

        end_conditions
    }

    pub fn build(&self, line_force_model: &LineForceModel) -> CirculationSmoothing {
        let end_conditions = self.end_conditions(line_force_model);
        let nr_wings = line_force_model.nr_wings();
        let wing_span_lengths = line_force_model.span_lengths();
        
        match &self.smoothing_type {
            SmoothingTypeBuilder::Gaussian(settings_builder) => {
                let mut settings_vector: Vec<GaussianSmoothing<f64>> = Vec::with_capacity(nr_wings);

                for wing_index in 0..nr_wings {
                    let smoothing_length = wing_span_lengths[wing_index] * settings_builder.smoothing_length_factor;

                    settings_vector.push(
                        GaussianSmoothing {
                            smoothing_length,
                            end_conditions: end_conditions[wing_index],
                            number_of_end_insertions: None,
                            delta_x_factor_end_insertions: 0.5,
                            number_of_end_points_to_interpolate: settings_builder.number_of_end_points_to_interpolate
                        }
                    );

                }

                CirculationSmoothing {
                    smoothing_type: SmoothingType::Gaussian(settings_vector),
                    prescribed_to_subtract_before_smoothing: self.prescribed_to_subtract_before_smoothing.clone()
                }
            },
            SmoothingTypeBuilder::CubicPolynomial(settings_builder) => {
                let mut settings_vector: Vec<CubicPolynomialSmoothing<f64>> = Vec::with_capacity(nr_wings);

                for wing_index in 0..nr_wings {
                    settings_vector.push(
                        CubicPolynomialSmoothing {
                            window_size: settings_builder.window_size, 
                            end_conditions: end_conditions[wing_index]
                        }
                    );
                }

                CirculationSmoothing {
                    smoothing_type: SmoothingType::CubicPolynomial(settings_vector),
                    prescribed_to_subtract_before_smoothing: self.prescribed_to_subtract_before_smoothing.clone()
                }
            }
        }
        
    }
}