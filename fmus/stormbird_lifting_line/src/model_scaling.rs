#[derive(Clone, Debug, Copy)]
pub struct ModelScaling {
    pub scale: f64
}

impl ModelScaling {
    /// Function to scale time from model scale to full scale
    pub fn upscale_time(&self, time_value: f64) -> f64 {
        time_value * self.scale.sqrt()
    }
}