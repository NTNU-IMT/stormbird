# Results from simulations

Each time step in a simulation produces a *result struct* containing the calculated values. The internal Rust structure is presented below. This data is serialized directly to JSON when written to file.

## Main result struct
```rust
pub struct SimulationResult {
    pub ctrl_points: Vec<SpatialVector<3>>,
    pub force_input: SectionalForcesInput,
    pub sectional_forces: SectionalForces,
    pub integrated_forces: Vec<IntegratedValues>,
    pub integrated_moments: Vec<IntegratedValues>,
    pub iterations: usize,
    pub residual: f64,
}
```

## Sectional force input
```rust
/// Input data to calculate sectional forces.
pub struct SectionalForcesInput {
    /// Circulation strength of each line element
    pub circulation_strength: Vec<f64>,
    /// The *felt* velocity at each control point, meaning the velocity of the fluid from the 
    /// perspective of the wings, **not** the velocity of the wings themselves.  
    pub velocity: Vec<SpatialVector<3>>,
    /// The estimated angle of attack at each control point.
    pub angles_of_attack: Vec<f64>,
    /// The *felt* acceleration at each control point, meaning the acceleration of the fluid from 
    /// the perspective of the wings, **not** the acceleration of the wings themselves.
    pub acceleration: Vec<SpatialVector<3>>,
    /// The change in angle of attack at each control point as a function of time. 
    pub angles_of_attack_derivative: Vec<f64>,
    /// The rotational velocity of the entire system. Primarily relevant for gyroscopic effects.
    pub rotation_velocity: SpatialVector<3>,
}
```

## Sectional forces
```rust
/// Structures used to store sectional forces from simulations.
pub struct SectionalForces {
    /// Forces due to the circulation on a line element. Computed from the lift part of the 
    /// sectional model.
    pub circulatory: Vec<SpatialVector<3>>,
    /// Forces due to the two dimensional drag on a line element. 
    /// 
    /// **Note**: this is often the viscous drag, but not always. In can also include three 
    /// dimensional effects on the drag, if the model is executed with a simplified approach, for 
    /// instance when neglecting the *self-induced* velocities.
    pub sectional_drag: Vec<SpatialVector<3>>,
    /// Added mass forces on the line element.
    pub added_mass: Vec<SpatialVector<3>>,
    /// Forces due to gyroscopic effects on the line element.
    pub gyroscopic: Vec<SpatialVector<3>>,
    /// Total forces
    pub total: Vec<SpatialVector<3>>,
}
```

## Integrated values
```rust
/// Integrated values representing either forces or moments.
pub struct IntegratedValues {
    pub circulatory: SpatialVector<3>,
    pub sectional_drag: SpatialVector<3>,
    pub added_mass: SpatialVector<3>,
    pub gyroscopic: SpatialVector<3>,
    pub total: SpatialVector<3>,
}

```