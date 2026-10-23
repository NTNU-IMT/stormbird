# Force calculations

An important job for the line force model is to compute forces on each line element, as a function of the local flow variables. The results from these calculations are available in the result-structures from a simulation. A view of the result structure is shown as Rust code below. There is also a Python interface to this structure where the same fields are available.

```rust
pub struct SimulationResult {
    pub time: f64,
    pub ctrl_points: Vec<SpatialVector>,
    pub solver_input_ctrl_points_velocity: Vec<SpatialVector>,
    pub force_input: SectionalForcesInput,
    pub sectional_forces: SectionalForces,
    pub integrated_forces: Vec<IntegratedValues>,
    pub integrated_moments: Vec<IntegratedValues>,
    pub input_power: Vec<f64>,
    pub iterations: usize,
    pub residual: f64,
    pub wing_indices: Vec<Range<usize>>,
    pub rigid_body_motion: RigidBodyMotion,
}
```

## Sectional vs integrated
The forces on a line force model are specified both as *sectional values* and as *integrated values*. As the name suggests, the sectional forces are forces acting on each individual section in the line force model. That is, the force acting on a single line element. The integrated forces and moments are the sum of sectional forces for each wing in the line force model. That is, the length of the integrated forces and moments vectors will be equal to the number of wings. Line elements that are marked as *virtual* in the [wing builder](./building_line_model.md) are not included in the integrated values.

## Force types
There are four different force types in Stormbird, together with a gyroscopic moment, which are estimated from different methods. Each has a dedicated section below. The circulation forces, the viscous lift and the sectional drag are computed from the section models, while the added mass and the gyroscopic moment are computed from models that are set for each wing in the [wing builder](./building_line_model.md). Empirical corrections of the forces are described in the [force corrections](#force-corrections) section.

### 1 - Circulation forces
The first, and usually most important, force component is the force that arises from the circulation on each line segment. The circulation strength is estimated directly from the local velocity and sectional model, as described [here](./circulation_strength.md). The direction of the force is always normal to the local velocity.

The resulting force from the circulation strength is therefore just the "lift" on each line segment, but in the coordinate system of the local flow. Due to the presence of induced velocities, the sectional lift may cause both lift and drag relative to the incoming free stream. Circulation forces are, in other words, the combination of lift and lift-induced drag from the simulation.

### 2 - Viscous lift

The sectional models for foils divide the lift coefficient into a pre-stall part and a post-stall part, as explained in the [foil model chapter](../sectional_models/foil_model.md). By default, only the pre-stall part is used when calculating the circulation strength. The post-stall part is instead calculated as a separate force, labeled *viscous lift*, which acts normal to the local velocity, but which does not affect the lift-induced velocities. This force type is always zero for the rotating cylinder model.

Whether the post-stall lift should be included in the circulation instead is controlled by the `ForceCalculationSettings` structure, which is set in the [line force model builder](./building_line_model.md):

```rust
pub struct ForceCalculationSettings {
    pub include_viscous_lift_in_the_circulation: bool
}
```

The default value is `false`. If it is set to `true`, the post-stall lift is included in the circulation strength, and the viscous lift force will always be zero.

### 3 - Sectional drag

Sectional drag is, usually[^sectional_drag_note], the viscous drag on each line segment. It is calculated directly from the drag function in the [sectional model](../sectional_models/sectional_models_intro.md). The direction is always parallel to the local flow.

The sectional drag also includes the drag from the [lift-induced drag correction](#lift-induced-drag-correction), if it is used.

[^sectional_drag_note]: It is perfectly fine to include more than the viscous drag in the sectional drag model, if, for instance, it is necessary to add some empirical corrections to the total drag estimate. What exactly the sectional drag represents from a purely physical point of view must be decided on when setting up the model by the user. 

### 4 - Added mass

[Added mass forces](https://en.wikipedia.org/wiki/Added_mass) are the forces that are proportional to the acceleration of the flow relative to the wing. They are computed with a strip theory model, set with the `added_mass` field in the wing builder:

```rust
pub struct AddedMass {
    pub normal_coefficient: f64,
    pub chordwise_coefficient: f64,
}
```

The added mass of each line element is given relative to a reference mass, which is the added mass of a flat plate accelerating normal to its chord in two-dimensional potential flow:

\\[
    m\_{ref} = \rho \frac{\pi c^2}{4} \Delta s
\\]

where \\( \rho \\) is the density, \\( c \\) is the chord length and \\( \Delta s \\) is the length of the line element. The acceleration along the span line is removed, and the remaining part is divided in a part normal to the chord, \\( \mathbf{a}\_n \\), and a part along the chord, \\( \mathbf{a}\_c \\). The added mass force is then:

\\[
    \mathbf{F}\_{added \\, mass} = m\_{ref} \left( C\_{n} \mathbf{a}\_n + C\_{c} \mathbf{a}\_c \right)
\\]

where \\( C\_n \\) is `normal_coefficient` and \\( C\_c \\) is `chordwise_coefficient`. The acceleration is the *felt* acceleration of the flow, meaning that it is opposite to the acceleration of the wing, if the wing is moving in still air.

Both coefficients are zero by default, which means that the model is not used. Recommended values, based on two-dimensional potential flow theory, are given below:

| Case | `normal_coefficient` | `chordwise_coefficient` |
|---|---|---|
| Thin foil, e.g., a single element wing sail | 1.0 | 0.0 |
| Thick foil, with thickness ratio \\( t / c \\) | 1.0 | \\( (t / c)^2 \\) |
| Rotating cylinder, i.e., a rotor sail | 1.0 | 1.0 |

The values for the thick foil are exact for an elliptical section, and are a good approximation for other foil shapes. For instance, a foil with a thickness ratio of 18% gets a chordwise coefficient of 0.03, which can usually be neglected. For a rotating cylinder, the chord length is the diameter, so the reference mass is the mass of the displaced fluid, and the force is the same for acceleration in all directions normal to the span line. For multi-element foils, the chord length should be the total chord length of all the elements.

The model is a strip theory, where each line element is treated as a two-dimensional section. The real added mass of a wing with a finite span is lower, in particular close to the tips and for wings with a low aspect ratio. This is not corrected for automatically, but can be accounted for by reducing the coefficients.

### 5 - Gyroscopic moment

The gyroscopic moment is only applicable to rotor sails. It is the moment due to [gyroscopic precession](https://en.wikipedia.org/wiki/Precession), when the spinning rotor is rotated by the motion of the ship, for instance in roll or pitch. It is computed with a model that is set with the `gyroscopic_moment` field in the wing builder:

```rust
pub struct GyroscopicMoment {
    pub moment_of_inertia: f64,
}
```

The `moment_of_inertia` is the total moment of inertia of the rotor about its axis of rotation, in kg m^2. It is zero by default, which means that the model is not used. The model can only be used together with the [rotating cylinder](../sectional_models/rotating_cylinder.md) section model, as the internal state of the wing is then the revolutions per second of the rotor, \\( n \\). Setting a non-zero moment of inertia for other section models gives an error.

The angular momentum of the rotor is \\( \mathbf{L} = I \\, 2 \pi n \\, \mathbf{e} \\), where \\( I \\) is the moment of inertia and \\( \mathbf{e} \\) is the direction of the span line. The rotational speed of the rotor is assumed to be much larger than the rotational velocity of the line force model, \\( \boldsymbol{\Omega} \\), which gives the moment from the rotor on its supports:

\\[
    \mathbf{M}\_{gyroscopic} = \mathbf{L} \times \boldsymbol{\Omega}
\\]

The moment is distributed over the line elements in proportion to their length, so that a span varying internal state is taken into account. Virtual line elements get no moment. The gyroscopic effect is a pure moment, and is therefore not part of the forces. It is instead added directly to the integrated moments, see the [force results structures](#force-results-structures) section below.

The moment of inertia should include all the rotating parts, i.e., the cylinder and the end disk. If it is not known from the manufacturer, it can be estimated from the mass and the radius of the parts. A thin-walled cylinder with mass \\( m \\) and radius \\( R \\) has a moment of inertia of \\( m R^2 \\), while a uniform disk has \\( m R^2 / 2 \\).

## Force corrections

The lifting line and actuator line methods are simplified models of the flow around the wings, and the results may need to be adjusted to fit experimental data or high-fidelity simulations. Empirical corrections of the forces can therefore be set for each wing, through the `force_correction` field in the wing builder:

```rust
pub struct ForceCorrection {
    pub lift_induced_drag_correction: f64,
}
```

All corrections are zero by default, which means that they are not used. More corrections may be added to this structure in the future.

### Lift-induced drag correction

The lift-induced drag correction adds a drag force to each line element, with a drag coefficient that is proportional to the square of the lift coefficient:

\\[
    \Delta C\_D = k\_{i} C\_L^2
\\]

where \\( k\_i \\) is `lift_induced_drag_correction`. The lift coefficient is computed from the total lift force on the line element, i.e., the sum of the [circulation force](#1---circulation-forces) and the [viscous lift](#2---viscous-lift). The drag force is parallel to the local velocity, and is included in the [sectional drag](#3---sectional-drag). The correction is applied after the circulation strength is computed, and does therefore not affect the lift or the induced velocities.

A positive value increases the drag, while a negative value decreases it. As the lift-induced drag of a wing is approximately \\( C\_L^2 / (\pi A) \\), where \\( A \\) is the aspect ratio, the correction can be thought of as a change in the *effective* aspect ratio of the wing. To get the lift-induced drag of a wing with an effective aspect ratio of \\( A\_{eff} \\) rather than \\( A \\), the factor should approximately be:

\\[
    k\_i = \frac{1}{\pi A\_{eff}} - \frac{1}{\pi A}
\\]

The value must be used with care, as a negative value can give a negative total drag if it is too large.

### Virtual line segments

[Virtual line segments](./building_line_model.md#wing-builder) can also be used to alter the forces in a way that is similar to the lift-induced drag correction. A virtual segment at the end of a wing makes the wing effectively longer in the simulation, while the forces on the virtual segment are not included in the integrated forces. This increases the effective aspect ratio of the wing, for instance to model the effect of an end plate, or of a wing standing close to the deck of a ship. As the circulation is then non-zero at the end of the real part of the wing, the lift also increases slightly, and the lift-induced drag is reduced.

The two methods therefore largely do the same thing, but there are some differences:

- Virtual line segments only work one way. They can increase the effective aspect ratio, and thereby reduce the lift-induced drag, but they can not increase the drag. The lift-induced drag correction is the only way to increase the drag.
- Virtual line segments affect the circulation distribution, and therefore also the lift and the interaction with other wings, while the lift-induced drag correction only affects the drag.
- Virtual line segments add more line elements to the simulation, which increases the computational cost, while the lift-induced drag correction is practically free.

## Force results structures

To allow for a comparison between the force types - for instance in debugging situations - each force type is stored separately, in addition to the total force / moment. The structures used to store this data are shown below:

```rust
pub struct SectionalForces {
    pub circulatory: Vec<SpatialVector>,
    pub viscous_lift: Vec<SpatialVector>,
    pub sectional_drag: Vec<SpatialVector>,
    pub added_mass: Vec<SpatialVector>,
    pub gyroscopic_moment: Vec<SpatialVector>,
    pub total: Vec<SpatialVector>,
    pub coordinate_system: CoordinateSystem,
}
```

The `gyroscopic_moment` is the moment from each line element, and is not included in the `total` force.
```rust
pub struct IntegratedValues {
    pub circulatory: SpatialVector,
    pub viscous_lift: SpatialVector,
    pub sectional_drag: SpatialVector,
    pub added_mass: SpatialVector,
    pub gyroscopic: SpatialVector,
    pub total: SpatialVector,
}
```

The integrated moments are computed as the sum of the cross product between the control point and the sectional force on each line element, with the gyroscopic moment added directly. The `gyroscopic` field is therefore always zero for the integrated forces.


