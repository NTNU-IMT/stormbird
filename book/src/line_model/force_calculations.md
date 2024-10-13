# Force calculations

An important job for the line force model is to compute forces on each line element. The results from these calculations are available in the result-structures from a simulation, as specified [here](./results.md). This chapter specifies how the forces are calculated. 

## Sectional vs integrated
The forces on a line force model is specified both as *sectional values* and as *integrated values*. 

As the name suggest, the sectional forces are forces acting on each individual section in the line force model. That is, the force acting on a single line element.

The integrated forces are

## Force types
There are four different force types in Stormbird, which are estimated from different methods. Each force type has a dedicated section below

### 1 - Circulation forces
The first, and usually most important, force component is the force that arise from the circulation on each line segment. The circulation strength is estimated directly from the local velocity and sectional model, as described [here](./circulation_strength.md). The direction of the force is always normal to the local velocity.

The resulting force from the circulation strength is therefore just the "lift" on each line segment, but in the coordinate system of the local flow. Due to the presence of induced velocities, the sectional lift may cause both lift and drag relative to the incoming free stream. Circulation forces are, in other words, the combination of lift and lift-induced drag from the simulation.

### 2 - Sectional drag

Sectional drag is, usually[^sectional_drag_note], the viscous drag on each line segment. It is calculated directly from the drag function in the [sectional model](../sectional_models/sectional_models_intro.md). The direction is always parallel to the local flow.

[^sectional_drag_note]: It is perfectly fine to include more than the viscous drag in the sectional drag model, if, for instance, it is necessary to add some empirical corrections to the total drag estimate. What exactly the sectional drag represents from a purely physical point of view must be decided on when setting up the model by the user. 

### 3 - Added mass

[Added mass forces](https://en.wikipedia.org/wiki/Added_mass) are the forces that are proportional to the acceleration of the foil section. 

The magnitude of the force is determined by coefficients set in the sectional model. 

### 4 - Gyroscopic

This force should only be non-zero for models using the rotating cylinder sectional model.

## Force results structures

```rust
pub struct SectionalForces {
    pub circulatory: Vec<SpatialVector<3>>,
    pub sectional_drag: Vec<SpatialVector<3>>,
    pub added_mass: Vec<SpatialVector<3>>,
    pub gyroscopic: Vec<SpatialVector<3>>,
    pub total: Vec<SpatialVector<3>>,
}
```
```rust
pub struct IntegratedValues {
    pub circulatory: SpatialVector<3>,
    pub sectional_drag: SpatialVector<3>,
    pub added_mass: SpatialVector<3>,
    pub gyroscopic: SpatialVector<3>,
    pub total: SpatialVector<3>,
}
```

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
