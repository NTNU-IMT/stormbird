use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// A simple model of a fan system, primarily used to model suction sails. The main point of this 
/// model is to be able to compute the flow through a suction sail based on e.g. input power. This 
/// may be required to find the right foil profile settings, of the foil profile is set to depend on 
/// the non-dimensional flow coefficient.
pub struct FanSystem {
    /// A constant drive efficiency for the fan system. A real fan will have a variable efficiency
    /// that depends on load and rotational speed. However, this serves as a first draft, to get a 
    /// simple model working 
    pub drive_efficiency: Float,
    /// Fraction of the chords that has porous openings
    pub porous_fraction: Float,
    /// Empirical coefficient determining the relationship between pressure drop and flow through
    /// a station
    pub loss_coefficient: Float
}

impl FanSystem {
    pub fn compute_mass_flow(&self, input_power: Float) -> Float {
        todo!()
    }

    pub fn external_pressure(&self) -> Vec<Float> {
        todo!()
    }

    
}