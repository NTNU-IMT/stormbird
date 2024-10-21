# Force calculations

An important job for the line force model is to compute forces on each line element, as a function of the local flow variables. The results from these calculations are available in the result-structures from a simulation. A view of the result structure is shown as Rust code below. There is also a Python interface to this structure where the same fields are available.

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

## Sectional vs integrated
The forces on a line force model is specified both as *sectional values* and as *integrated values*. As the name suggest, the sectional forces are forces acting on each individual section in the line force model. That is, the force acting on a single line element. The integrated forces and moments are the sum of sectional forces for each wing in the line force model. That is, the length of the integrated forces and moments vectors will be equal to the number of wings.

## Force types
There are four different force types in Stormbird, which are estimated from different methods. Each force type has a dedicated section below

### 1 - Circulation forces
The first, and usually most important, force component is the force that arise from the circulation on each line segment. The circulation strength is estimated directly from the local velocity and sectional model, as described [here](./circulation_strength.md). The direction of the force is always normal to the local velocity.

The resulting force from the circulation strength is therefore just the "lift" on each line segment, but in the coordinate system of the local flow. Due to the presence of induced velocities, the sectional lift may cause both lift and drag relative to the incoming free stream. Circulation forces are, in other words, the combination of lift and lift-induced drag from the simulation.

### 2 - Sectional drag

Sectional drag is, usually[^sectional_drag_note], the viscous drag on each line segment. It is calculated directly from the drag function in the [sectional model](../sectional_models/sectional_models_intro.md). The direction is always parallel to the local flow.

[^sectional_drag_note]: It is perfectly fine to include more than the viscous drag in the sectional drag model, if, for instance, it is necessary to add some empirical corrections to the total drag estimate. What exactly the sectional drag represents from a purely physical point of view must be decided on when setting up the model by the user. 

### 3 - Added mass

[Added mass forces](https://en.wikipedia.org/wiki/Added_mass) are the forces that are proportional to the acceleration of the foil section. The magnitude of the force is determined by the added mass coefficients set in the sectional model and the acceleration. The foil-models only result in added mass forces due to acceleration in the direction normal to the chord vectors. For the rotating cylinder, the added 

**Note**: for now, the default value of the added mass coefficient is set to zero. This is because the feature is currently lacking a proper test case. Must therefore be used with care!

### 4 - Gyroscopic

The gyroscopic force/moment is only applicable to rotor sails. It is the [gyroscopic precision](https://en.wikipedia.org/wiki/Precession) due to the rotation of the cylinders. It is dependent on the 2D moment of inertia, which needs to be specified for the sectional model in order to make this force non-zero. 

## Force results structures

To allow for a comparison between the force types - for instance in debugging situations - each force type is stored separately, in addition to the total force / moment. The structures used to store this data is shown below:

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


