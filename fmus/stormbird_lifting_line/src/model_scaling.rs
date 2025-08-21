use stormath::spatial_vector::SpatialVector;

#[derive(Clone, Debug, Copy)]
pub struct ModelScaling {
    pub scale: f64
}

impl ModelScaling {
    /// Function to scale time from model scale to full scale
    pub fn upscale_time(&self, time_value: f64) -> f64 {
        time_value * self.scale.sqrt()
    }

    /// Function to scale time from full scale to model scale
    pub fn downscale_time(&self, time_value: f64) -> f64 {
        time_value / self.scale.sqrt()
    }

    pub fn upscale_length(&self, length_vector: SpatialVector<3>) -> SpatialVector<3> {
        length_vector * self.scale
    }

    pub fn downscale_length(&self, length_vector: SpatialVector<3>) -> SpatialVector<3> {
        length_vector / self.scale
    }

    pub fn upscale_velocity(&self, velocity_vector: SpatialVector<3>) -> SpatialVector<3> {
        velocity_vector * self.scale.sqrt()
    }

    pub fn downscale_velocity(&self, velocity_vector: SpatialVector<3>) -> SpatialVector<3> {
        velocity_vector / self.scale.sqrt()
    }

    pub fn upscale_force(&self, force_vector: SpatialVector<3>) -> SpatialVector<3> {
        self.scale.powi(3) * force_vector
    }

    pub fn downscale_force(&self, force_vector: SpatialVector<3>) -> SpatialVector<3> {
        force_vector / self.scale.powi(3)
    }

    pub fn upscale_moment(&self, moment_vector: SpatialVector<3>) -> SpatialVector<3> {
        self.scale.powi(4) * moment_vector
    }

    pub fn downscale_moment(&self, moment_vector: SpatialVector<3>) -> SpatialVector<3> {
        moment_vector / self.scale.powi(4)
    }
}