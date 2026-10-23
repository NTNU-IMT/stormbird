// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! The internal state of a wing, which can either be constant or vary along the span.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::interpolation::linear_interpolation;

/// The non-dimensional start of the span of a wing
pub const SPAN_START: Float = -0.5;
/// The non-dimensional end of the span of a wing
pub const SPAN_END: Float = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "SpanVariationInput")]
/// A variation of a value along the span of a wing. The locations are given as non-dimensional
/// span positions, running from -0.5 at one end of the wing to 0.5 at the other.
///
/// The variation does not need to match the resolution of the line elements, as it is interpolated
/// to the location of each line element, or averaged and integrated over parts of the span.
pub enum SpanVariation {
    /// The value is linearly interpolated between the locations, and constant outside of them.
    Linear {
        /// Non-dimensional span locations of the values
        locations: Vec<Float>,
        /// The values at each location
        values: Vec<Float>,
    },
    /// The value is constant within each zone, defined by the boundaries between them. There must
    /// be one more boundary than values. Outside of the boundaries, the value of the closest zone
    /// is used.
    PiecewiseConstant {
        /// Non-dimensional span locations of the boundaries between the zones
        boundaries: Vec<Float>,
        /// The value in each zone
        values: Vec<Float>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Unchecked version of the span variation, used for deserialization
enum SpanVariationInput {
    Linear {
        locations: Vec<Float>,
        values: Vec<Float>,
    },
    PiecewiseConstant {
        boundaries: Vec<Float>,
        values: Vec<Float>,
    },
}

impl TryFrom<SpanVariationInput> for SpanVariation {
    type Error = String;

    fn try_from(input: SpanVariationInput) -> Result<Self, Self::Error> {
        match input {
            SpanVariationInput::Linear { locations, values } => {
                Self::new_linear(locations, values)
            },
            SpanVariationInput::PiecewiseConstant { boundaries, values } => {
                Self::new_piecewise_constant(boundaries, values)
            },
        }
    }
}

impl SpanVariation {
    /// Creates a linear span variation, and checks that the input is valid
    pub fn new_linear(locations: Vec<Float>, values: Vec<Float>) -> Result<Self, String> {
        if locations.is_empty() {
            return Err("A linear span variation must have at least one value".to_string());
        }

        if locations.len() != values.len() {
            return Err(format!(
                "A linear span variation must have the same number of locations and values. \
                Locations: {}, values: {}",
                locations.len(), values.len()
            ));
        }

        Self::check_increasing(&locations, "locations")?;

        Ok(Self::Linear { locations, values })
    }

    /// Creates a piecewise constant span variation, and checks that the input is valid
    pub fn new_piecewise_constant(boundaries: Vec<Float>, values: Vec<Float>) -> Result<Self, String> {
        if values.is_empty() {
            return Err("A piecewise constant span variation must have at least one value".to_string());
        }

        if boundaries.len() != values.len() + 1 {
            return Err(format!(
                "A piecewise constant span variation must have one more boundary than values. \
                Boundaries: {}, values: {}",
                boundaries.len(), values.len()
            ));
        }

        Self::check_increasing(&boundaries, "boundaries")?;

        Ok(Self::PiecewiseConstant { boundaries, values })
    }

    fn check_increasing(data: &[Float], name: &str) -> Result<(), String> {
        if data.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(format!("The {} of a span variation must be strictly increasing", name));
        }

        Ok(())
    }

    /// The value at a non-dimensional span location
    pub fn value_at(&self, location: Float) -> Float {
        match self {
            Self::Linear { locations, values } => {
                linear_interpolation(location, locations, values)
            },
            Self::PiecewiseConstant { boundaries, values } => {
                let zone_index = boundaries[1..boundaries.len() - 1]
                    .iter()
                    .filter(|&&boundary| location >= boundary)
                    .count();

                values[zone_index]
            },
        }
    }

    /// The locations where the variation changes behavior, i.e., where the value is not linear
    /// across the location
    fn break_points(&self) -> &[Float] {
        match self {
            Self::Linear { locations, .. } => locations,
            Self::PiecewiseConstant { boundaries, .. } => boundaries,
        }
    }

    /// The exact integral of the value, or the absolute value, between two non-dimensional
    /// locations. The variation is linear between the break points, so the integral is computed
    /// as a sum of exact integrals over each linear part.
    fn integrate(&self, start: Float, end: Float, absolute_value: bool) -> Float {
        if end <= start {
            return 0.0;
        }

        let mut points: Vec<Float> = vec![start];

        points.extend(
            self.break_points().iter().copied().filter(|&point| point > start && point < end)
        );

        points.push(end);

        let mut integral = 0.0;

        for pair in points.windows(2) {
            let (a, b) = (pair[0], pair[1]);

            // Evaluate inside the interval, so that the value of the right zone is used at the
            // boundaries of a piecewise constant variation.
            let (value_a, value_b) = match self {
                Self::Linear { .. } => (self.value_at(a), self.value_at(b)),
                Self::PiecewiseConstant { .. } => {
                    let value = self.value_at(0.5 * (a + b));

                    (value, value)
                },
            };

            integral += if absolute_value {
                Self::integral_of_absolute_linear(value_a, value_b, b - a)
            } else {
                0.5 * (value_a + value_b) * (b - a)
            };
        }

        integral
    }

    /// The integral of the absolute value of a linear function between two points
    fn integral_of_absolute_linear(value_a: Float, value_b: Float, length: Float) -> Float {
        if value_a * value_b >= 0.0 {
            0.5 * (value_a.abs() + value_b.abs()) * length
        } else {
            // The function crosses zero between the points
            let zero_crossing = value_a.abs() / (value_a.abs() + value_b.abs()) * length;

            0.5 * value_a.abs() * zero_crossing + 0.5 * value_b.abs() * (length - zero_crossing)
        }
    }

    /// The integral of the value between two non-dimensional span locations
    pub fn integral(&self, start: Float, end: Float) -> Float {
        self.integrate(start, end, false)
    }

    /// The integral of the absolute value between two non-dimensional span locations
    pub fn integral_of_absolute_value(&self, start: Float, end: Float) -> Float {
        self.integrate(start, end, true)
    }

    /// The mean value between two non-dimensional span locations
    pub fn mean(&self, start: Float, end: Float) -> Float {
        if end - start <= 0.0 {
            return self.value_at(start);
        }

        self.integral(start, end) / (end - start)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The internal state of a wing, which can either be constant along the span, or vary along the
/// span. The meaning of the value depends on the section model of the wing.
pub enum WingInternalState {
    /// The same value along the whole span
    Constant(Float),
    /// A value that varies along the span
    SpanVarying(SpanVariation),
}

impl Default for WingInternalState {
    fn default() -> Self {
        Self::Constant(0.0)
    }
}

impl From<Float> for WingInternalState {
    fn from(value: Float) -> Self {
        Self::Constant(value)
    }
}

impl WingInternalState {
    /// The value at a non-dimensional span location
    pub fn value_at(&self, location: Float) -> Float {
        match self {
            Self::Constant(value) => *value,
            Self::SpanVarying(variation) => variation.value_at(location),
        }
    }

    /// The mean value between two non-dimensional span locations
    pub fn mean(&self, start: Float, end: Float) -> Float {
        match self {
            Self::Constant(value) => *value,
            Self::SpanVarying(variation) => variation.mean(start, end),
        }
    }

    /// The mean value over the whole span. Used when a single value is needed to represent the
    /// state of the whole wing, for instance by the controller.
    pub fn representative_value(&self) -> Float {
        self.mean(SPAN_START, SPAN_END)
    }

    /// The integral of the value between two non-dimensional span locations, multiplied with the
    /// span length. If the value is given per unit span length, e.g., power per span, this gives
    /// the total for that part of the span.
    pub fn span_integral(&self, start: Float, end: Float, span_length: Float) -> Float {
        match self {
            Self::Constant(value) => value * (end - start) * span_length,
            Self::SpanVarying(variation) => variation.integral(start, end) * span_length,
        }
    }

    /// The same as [`span_integral`](WingInternalState::span_integral), but for the absolute value
    pub fn span_integral_of_absolute_value(&self, start: Float, end: Float, span_length: Float) -> Float {
        match self {
            Self::Constant(value) => value.abs() * (end - start) * span_length,
            Self::SpanVarying(variation) => {
                variation.integral_of_absolute_value(start, end) * span_length
            },
        }
    }

    /// The mean of the absolute value over the whole span
    pub fn representative_absolute_value(&self) -> Float {
        self.span_integral_of_absolute_value(SPAN_START, SPAN_END, 1.0) / (SPAN_END - SPAN_START)
    }

    /// Returns true if the internal state is constant along the span
    pub fn is_constant(&self) -> bool {
        matches!(self, Self::Constant(_))
    }
}

#[derive(Debug, Clone)]
/// Division of the internal volume of a wing into sealed compartments along the span, for instance
/// for a suction sail with several fans separated by bulkheads. Each compartment has its own 
/// uniform internal pressure, with no flow between the compartments.
///
/// The compartments are defined by the non-dimensional span locations of their boundaries, 
/// including the ends of the wing at -0.5 and 0.5.
pub struct InternalCompartments {
    boundaries: Vec<Float>,
}

impl Default for InternalCompartments {
    /// A single compartment covering the whole span
    fn default() -> Self {
        Self { boundaries: vec![SPAN_START, SPAN_END] }
    }
}

impl InternalCompartments {
    /// Creates compartments from the boundaries, and checks that they are valid
    pub fn new(boundaries: Vec<Float>) -> Result<Self, String> {
        if boundaries.len() < 2 {
            return Err(
                "The internal compartment boundaries must at least contain the two ends of the wing"
                .to_string()
            );
        }

        let ends_are_correct = (boundaries[0] - SPAN_START).abs() < 1e-12 &&
            (boundaries[boundaries.len() - 1] - SPAN_END).abs() < 1e-12;

        if !ends_are_correct {
            return Err(format!(
                "The internal compartment boundaries must start at {} and end at {}",
                SPAN_START, SPAN_END
            ));
        }

        if boundaries.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err("The internal compartment boundaries must be strictly increasing".to_string());
        }

        Ok(Self { boundaries })
    }

    /// Returns the non-dimensional span locations of the boundaries between the compartments,
    /// including the ends of the wing
    pub fn boundaries(&self) -> &[Float] {
        &self.boundaries
    }

    /// Returns the number of compartments
    pub fn nr_compartments(&self) -> usize {
        self.boundaries.len() - 1
    }

    /// The non-dimensional start and end of a compartment
    pub fn range(&self, compartment_index: usize) -> (Float, Float) {
        (self.boundaries[compartment_index], self.boundaries[compartment_index + 1])
    }

    /// The index of the compartment that contains a non-dimensional span location. A boundary 
    /// belongs to the compartment above it.
    pub fn index_of(&self, location: Float) -> usize {
        self.boundaries[1..self.boundaries.len() - 1]
            .iter()
            .filter(|&&boundary| location >= boundary)
            .count()
    }
}
