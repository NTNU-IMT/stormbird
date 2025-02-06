use super::*;

impl StormbirdLiftingLine {
    /// Function that applies scaling, from model input scale to full-scale, based on the model scale factor.
    /// The functionality is turned off if the model scale factor is less than or equal to zero.
    pub fn upscaled_time_value(&self, time_value: f64) -> f64 {
        if self.parameters.model_scale_factor > 0.0 {
            time_value / self.parameters.model_scale_factor.sqrt()
        } else {
            time_value
        }
    }

    pub fn upscaled_length_value(&self, length_value: f64) -> f64 {
        if self.parameters.model_scale_factor > 0.0 {
            length_value / self.parameters.model_scale_factor
        } else {
            length_value
        }
    }

    pub fn upscaled_velocity_value(&self, velocity_value: f64) -> f64 {
        if self.parameters.model_scale_factor > 0.0 {
            velocity_value / self.parameters.model_scale_factor.sqrt()
        } else {
            velocity_value
        }
    }

    pub fn downscaled_force_value(&self, force_value: f64) -> f64 {
        if self.parameters.model_scale_factor > 0.0 {
            force_value * self.parameters.model_scale_factor.powi(3)
        } else {
            force_value
        }
    }

    pub fn downscaled_moment_value(&self, moment_value: f64) -> f64 {
        if self.parameters.model_scale_factor > 0.0 {
            moment_value * self.parameters.model_scale_factor.powi(4)
        } else {
            moment_value
        }
    }
}