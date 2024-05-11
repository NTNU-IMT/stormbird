# Varying foil model

Structure capable of combining multiple foil models. The actual lift and drag can then depend on an internal parameter that is used to interpolate the foil models.

## Available fields
```rust
/// A foil profile where the parameters can vary depending on an internal state. 
/// 
/// The two typical use cases are to model foil sections that include a flap angle, or suction 
/// sails, where the foil section properties are dependent on the suction rate.
pub struct VaryingFoil {
    /// Typically flap angle or suction rate
    pub internal_state_data: Vec<f64>,
    /// The foil model that correspond to the internal state values
    pub foils_data: Vec<Foil>,
    /// The current internal state used when getting lift and drag. Defaults to zero
    pub current_internal_state: f64,
}
```