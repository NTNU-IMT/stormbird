# Empirical models

Pure empirical models that may be used on combination with other models in the library.

## Parametric model for wind loads on a ship superstructure

An empirical model for the forces acting on a superstructure. Included in the library to be able to make a "complete" model of the aerodynamics on a ship. The current method is very basic and consists of a slightly simplified version the [Blendermann model](https://doi.org/10.1016/0167-6105(94)90067-1).

MORE TO COME

```rust
pub struct BlendermannSuperstructureForces {
    pub frontal_area: f64,
    pub side_area: f64,
    pub center_of_effort: SpatialVector,
    pub resistance_coefficient: f64,
    pub side_force_coefficient: f64,
    pub coupling_factor: f64,
    pub density: f64,
```
