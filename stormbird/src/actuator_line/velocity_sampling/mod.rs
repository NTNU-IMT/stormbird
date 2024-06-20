// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

pub mod integral_sampling;
pub mod point_wise_sampling;

use serde::{Serialize, Deserialize};

use crate::vec3::Vec3;

use integral_sampling::{IntegralVelocitySamplingBuilder, IntegralVelocitySampling};
use point_wise_sampling::PointWiseVelocitySampling;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum VelocitySamplingBuilder {
    PointWise,
    Integral(IntegralVelocitySamplingBuilder),
}

impl Default for VelocitySamplingBuilder {
    fn default() -> Self {
        Self::PointWise
    }
}

impl VelocitySamplingBuilder {
    pub fn build(&self, nr_line_elements: usize) -> VelocitySampling {
        match self {
            Self::PointWise => VelocitySampling::PointWise(PointWiseVelocitySampling::new(nr_line_elements)),
            Self::Integral(builder) => VelocitySampling::Integral(builder.build(nr_line_elements)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VelocitySampling {
    PointWise(PointWiseVelocitySampling),
    Integral(IntegralVelocitySampling),
}

impl VelocitySampling {
    pub fn reset(&mut self) {
        match self {
            Self::PointWise(sampling) => sampling.reset(),
            Self::Integral(sampling) => sampling.reset(),
        }
    }

    pub fn ctrl_points_velocity(&self) -> Vec<Vec3> {
        match self {
            Self::PointWise(sampling) => sampling.ctrl_points_velocity.clone(),
            Self::Integral(sampling) => sampling.ctrl_points_velocity(),
        }
    }
}

