// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


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