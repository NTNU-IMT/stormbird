// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

use pyo3::prelude::*;

use pyo3::types::PyType;

use stormbird::line_force_model::builder::single_wing::WingBuilder as WingBuilderRust;
use stormbird::line_force_model::builder::LineForceModelBuilder as LineForceModelBuilderRust;

use crate::spatial_vector::SpatialVector;
use crate::section_models::SectionModel;
use super::LineForceModel;

#[pyclass]
#[derive(Clone)]
pub struct WingBuilder {
    pub data: WingBuilderRust,
}

#[pymethods]
impl WingBuilder {
    #[new]
    pub fn new(
        section_points: Vec<SpatialVector>, 
        chord_vectors: Vec<SpatialVector>, 
        section_model: SectionModel,
        non_zero_circulation_at_ends: [bool; 2],
        nr_sections: usize
    ) -> Self {
        WingBuilder {
            data: WingBuilderRust {
                section_points: section_points.iter().map(|v| v.data).collect(),
                chord_vectors: chord_vectors.iter().map(|v| v.data).collect(),
                section_model: section_model.data,
                non_zero_circulation_at_ends,
                nr_sections: Some(nr_sections),
            }
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct LineForceModelBuilder {
    pub data: LineForceModelBuilderRust,
}

#[pymethods]
impl LineForceModelBuilder {
    #[new]
    pub fn new(nr_sections: usize) -> Self {
        LineForceModelBuilder {
            data: LineForceModelBuilderRust::new(nr_sections),
        }
    }

    #[classmethod]
    pub fn new_from_string(_cls: &Bound<'_, PyType>, string: String) -> Self {
        Self {
            data: LineForceModelBuilderRust::new_from_string(&string).unwrap(),
        }
    }

    pub fn add_wing(&mut self, wing_builder: WingBuilder) {
        self.data.add_wing(wing_builder.data);
    }

    pub fn build(&self) -> LineForceModel {
        LineForceModel {
            data: self.data.build(),
        }
    }

    pub fn build_with_nr_sections(&self, nr_sections: usize) -> LineForceModel {
        LineForceModel {
            data: self.data.build_with_nr_sections(nr_sections),
        }
    }
}