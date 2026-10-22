# Lifting line wake

The vortex wakes from the wings are the most important part of a lifting line simulation. They are responsible for modeling how the velocity is affected by the wings themselves, i.e., the lift-induced velocities. How the induced velocities are calculated depends on several settings. Setting up a wake model is therefore done using settings structures, which contain both settings used directly by the final wake structures and settings used when initializing the wake structures. This section explains the two wake models, and what the available settings mean.

## Quasi-steady and dynamic wakes
Inside a simulation structure, the wake is represented as a `WakeData` structure that looks like this:

```rust
pub enum WakeData {
    Dynamic(DynamicWake),
    QuasiSteady(QuasiSteadyWakeSettings),
}
```

That is, the exact nature of the data depends on whether the wake is quasi-steady or dynamic:

- In the **quasi-steady** case, the wake is a set of horseshoe vortices, one for each line element, where the trailing vortices follow the velocity felt by the wings. The wake is rebuilt from scratch for every time step, so it has no memory of previous time steps, but the wake from the last time step is kept for [post-processing](#wake-export-and-induced-velocities). The quasi-steady wake is fast, but cannot model the dynamic variations in the lift-induced velocities, or a deformed wake shape.
- In the **dynamic** case, the wake consists of many vortex panels behind each line element, where both the strength and position of the panels evolve over time. This allows for modeling of the history of the wing motion, deformed wake shapes and viscous cores that change along the wake, but is more computationally expensive.

The wake settings are given as part of the [simulation settings](./simulation_overview.md), as a `QuasiSteadyWakeSettings` structure for quasi-steady simulations and a `DynamicWakeBuilder` structure for dynamic simulations.

### The frozen wake used by the solver
In both cases, the [solver](./solver.md) does not work with the wake directly. Instead, the wake is converted to a *frozen wake* at the beginning of every time step, where the shape of the wake is fixed. The induced velocity at each control point is then computed as:

\\[
    \mathbf{u}\_{i, j} = \mathbf{u}\_{fixed, j} + \sum\_k \mathbf{A}\_{j, k} \Gamma\_k
\\]

where \\( \Gamma\_k \\) is the unknown circulation strength on line element \\( k \\), \\( \mathbf{A}\_{j, k} \\) is the induced velocity at control point \\( j \\) from the vortices with unknown strength on line element \\( k \\), assuming unit strength, and \\( \mathbf{u}\_{fixed, j} \\) is the induced velocity from the parts of the wake where the strength is already known. For the quasi-steady wake, the whole horseshoe vortex has the unknown strength, so the fixed part is zero. For the dynamic wake, only the first panels behind the wings have unknown strength by default, while the rest of the panels have known strengths from previous time steps.

## Settings shared by both wake models

### Symmetry condition
The symmetry condition specifies if a symmetry plane should be assumed when calculating the lift-induced velocities:

```rust
pub enum SymmetryCondition {
    NoSymmetry,
    X,
    Y,
    Z,
}
```

The default is `NoSymmetry`. The other variants place a symmetry plane through the origin, normal to the given axis. For instance, `Z` gives a symmetry plane at \\( z = 0 \\), which is typically used to model the sea surface or a deck below the sails, so that the wings must be placed above or below this plane. The symmetry is modeled with the method of images: the induced velocity is computed both at the actual point and at the point mirrored in the symmetry plane, and the two are added, with the sign of the component normal to the plane flipped for the mirrored contribution. This gives zero velocity through the symmetry plane.

### Viscous core length
The value of the lift-induced velocity will go to infinity, according to potential theory, if one tries to evaluate it too close to a vortex line. This may cause problems when the vortex lines from one wing pass close to another wing, or close to the wake points of a dynamic wake. To handle this, Stormbird uses a viscous core model for all vortex lines. The induced velocity from a vortex line is multiplied by the factor:

\\[
    f\_{core} = \frac{d^2}{\sqrt{d^4 + r\_c^4}}
\\]

where \\( d \\) is the shortest distance from the point to the vortex line and \\( r\_c \\) is the viscous core length. The factor is close to 1 when the distance is much larger than the core length, and goes to zero when the distance goes to zero. The expression is taken from [Reid (2020)](https://digitalcommons.usu.edu/cgi/viewcontent.cgi?article=8982&context=etd).

The core length is given by the `ViscousCoreLength` enum:

```rust
pub enum ViscousCoreLength {
    Relative(f64),
    Absolute(f64),
    NoViscousCore,
}
```

- `Relative`: the core length is the given factor times the length of the bound vortex, i.e., the span length of the line element. For the quasi-steady wake, the average bound vortex length over all line elements is used. For the dynamic wake, the length of the line element the panel belongs to is used.
- `Absolute`: the core length is given directly, in meters.
- `NoViscousCore`: no viscous core is used. **Tip: turning it off may significantly speed up a simulation**, but may also cause instabilities if multiple sails are present.

The default value is `Relative(0.1)`, which means that the viscous core length is 10% of the bound vortex length.

## Quasi-steady wake

The quasi-steady wake is constructed on the fly for every time step with the following settings:

```rust
pub struct QuasiSteadyWakeSettings {
    pub wake_length_factor: f64,          // Default: 100.0
    pub symmetry_condition: SymmetryCondition,
    pub viscous_core_length: ViscousCoreLength,
}
```

Each line element gets one horseshoe vortex, which consists of five vortex lines with the same strength:

1) The bound vortex, along the line element itself.
2) Two short *elbow vortices*, one at each end of the bound vortex, going in the direction of the wake but normal to the bound vortex. Their length is 0.15 times the local chord length.
3) Two trailing vortices, going from the end of the elbow vortices in the direction of the wake.

The direction of the trailing vortices is the velocity felt by the wing at each end of the line element, i.e., the freestream velocity plus the velocity due to the motion of the wing. The induced velocities are not included, so the wake is straight. The length of the trailing vortices is given by the `wake_length_factor`, as a factor times the average chord length of all line elements. The default value is 100.0, which approximates a wake that extends infinitely far downstream.

The `symmetry_condition` and `viscous_core_length` are explained [above](#settings-shared-by-both-wake-models).

## Dynamic wake

### Structure of the wake
The dynamic wake consists of a structured grid of points behind each wing. For each point at the ends of the line elements (the *span points*), there is a row of wake points going downstream. The first point in each row is always at the span point on the wing itself, and the number of panels in each row is given by `nr_panels_per_line_element`. Each panel is a quadrilateral vortex ring, i.e., four vortex lines with the same strength, which is equivalent to a doublet panel with constant strength. The first panel behind each line element has the circulation strength of that line element.

### What happens in a time step
For each time step, the following happens with the wake:

1) The first points in the wake are moved to the current position of the wings, which may have moved since the last time step.
2) The rest of the wake points are moved downstream. Each point is moved to the previous position of the point in front of it, integrated with the velocity at that point over the time step: \\( \mathbf{p}\_{i}^{n+1} = \mathbf{p}\_{i-1}^{n} + \Delta t \\\, \mathbf{u}\_{i-1}^{n} \\). The velocity is the freestream velocity at the wake points, given as [input](./velocity_input.md) to the simulation, optionally with the induced velocities added.
3) The first free points behind the wings are placed at a fixed distance from the wings, in a direction given by the `first_wake_points_direction`. The last points in the wake are placed at a fixed distance from the second to last points, in the same direction as the previous panel. See the explanation of the parameters below.
4) The strength of the panels is shifted one panel downstream, so that the strengths follow the wake points.
5) The solver finds the new circulation strength on the wings, using the frozen wake.
6) The new circulation strength is applied to the first panels, and the velocity at the wake points is computed, to be used when moving the points in the next time step.

Since the points move one row per time step, and the first and last panels have fixed lengths, the length of the wake is approximately \\( (N - 2) \\\, U \\\, \Delta t \\) plus the lengths of the first and last panels, where \\( N \\) is the number of panels per line element, \\( U \\) is the velocity and \\( \Delta t \\) is the time step. The time step and the number of panels must therefore be chosen together, so that the wake becomes long enough.

At the first time step, the wake is initialized as a straight wake, with the average freestream velocity at the control points and the time step, so that it has the full length from the start. However, the strength of the panels starts at zero, and the strength from the wings is shifted downstream one panel per time step. It therefore takes about as many time steps as there are panels before the strength is established along the whole wake, unless the `steady_state_strength_update` is used.

### Available parameters
The settings for building a dynamic wake are shown below, with default values in the comments:

```rust
pub struct DynamicWakeBuilder {
    pub nr_panels_per_line_element: usize,                    // Default: 100
    pub viscous_core_length: ViscousCoreLength,               // Default: Relative(0.1)
    pub viscous_core_length_evolution: ViscousCoreLengthEvolution, // Default: Constant
    pub first_panel_relative_length: f64,                     // Default: 0.75
    pub last_panel_relative_length: f64,                      // Default: 1.0
    pub first_wake_points_direction: FirstWakePointsDirection, // Default: Freestream
    pub symmetry_condition: SymmetryCondition,                // Default: NoSymmetry
    pub ratio_of_wake_affected_by_induced_velocities: f64,    // Default: 0.0
    pub far_field_ratio: f64,                                 // Default: 5.0
    pub shape_damping_factor: f64,                            // Default: 0.0
    pub neglect_self_induced_velocities: bool,                // Default: false
    pub steady_state_strength_update: bool,                   // Default: false
    pub initial_relative_wake_length: f64,                    // Default: 50.0
}
```

Several of the lengths are given relative to a *representative chord length*, which is the average chord length of all line elements in the line force model.

#### Resolution and geometry
- `nr_panels_per_line_element`: The number of panels in the streamwise direction behind each line element. Must be at least 2. Together with the time step and the velocity, this determines the length of the wake, as explained above. The computational cost increases with the number of panels.
- `first_panel_relative_length`: The length of the first panel behind the wings, relative to the representative chord length. The first panel is treated separately from the rest of the wake, as its strength is unknown when solving for the circulation, and it is placed at a fixed distance from the wing, rather than being integrated in time.
- `last_panel_relative_length`: The length of the last panel in the wake, relative to the representative chord length. The last panel continues in the same direction as the previous panel.
- `first_wake_points_direction`: The direction of the first panels behind the wings. For the options based on velocity, the velocity component along the line elements is removed.
    - `Chord`: the panels follow the direction of the chord.
    - `Freestream`: the panels follow the velocity felt by the wings, i.e., the freestream velocity plus the motion of the wings, without induced velocities. This is the default.
    - `ActualVelocity`: the panels follow the velocity at the wake points from the previous time step, averaged between the wing and the first free wake points, which includes the induced velocities if they are calculated for the wake points. The freestream direction is used for the first time steps.
- `initial_relative_wake_length`: The length of the wake when the wake is built, relative to the representative chord length. This is only the shape of the wake before the first time step, as the wake is initialized again with the actual freestream velocity and time step at the first time step, as explained above.

#### Shape of the wake
- `ratio_of_wake_affected_by_induced_velocities`: The fraction of the wake, counted from the wings, where the induced velocities are included in the velocity used to move the wake points. The rest of the wake points only move with the freestream velocity. A value of 0.0 gives a wake that only follows the freestream, while 1.0 gives a fully deformable wake where all points are affected by the induced velocities. Calculating the induced velocities at the wake points is often the most expensive part of a dynamic simulation, as there are many wake points. The default is therefore 0.0. Including the induced velocities is mostly important when the lift coefficient is very high, such as for rotor sails, or when the wake from one sail passes close to another sail. The induced velocities on the wake points are only included after the first two time steps.
- `shape_damping_factor`: Damping of the motion of the wake points. The new position of a point is a weighted average of the old position and the position from the time integration: \\( \mathbf{p} = d \\\, \mathbf{p}\_{old} + (1 - d) \\\, \mathbf{p}\_{new} \\), where \\( d \\) is the damping factor. A value of 0.0 means no damping, while a value of 1.0 means that the wake points are fixed in space. Damping can be used to stabilize a deformable wake, as it limits how quickly the wake shape reacts to changes in the induced velocities.

#### Viscous core
- `viscous_core_length`: The viscous core length for the panels, as explained [above](#viscous-core-length).
- `viscous_core_length_evolution`: How the viscous core length changes along the wake. This is used to model that real vortices have a viscous core that grows when moving away from the wing. This can be very important when a sail stands directly in the wake of another sail. The options are:

```rust
pub enum ViscousCoreLengthEvolution {
    Constant,
    SinIncrease(SinIncreasedViscousCoreLength),
}

pub struct SinIncreasedViscousCoreLength {
    pub last_panel_value: ViscousCoreLength,  // Default: Relative(0.1)
    pub evolution_length_factor: f64,       // Default: 1.0
}
```

With `Constant`, which is the default, all panels use the `viscous_core_length`. With `SinIncrease`, the core length increases from the `viscous_core_length` at the first panel to the `last_panel_value` further downstream, following a quarter of a sine period:

\\[
    r\_c(i) = r\_{c, first} (1 - s\_i) + r\_{c, last} \\\, s\_i, \quad s\_i = \sin \left( \frac{\pi}{2} \min \left( \frac{i}{f (N - 1)}, 1 \right) \right)
\\]

where \\( i \\) is the streamwise index of the panel, \\( N \\) is the number of panels per line element and \\( f \\) is the `evolution_length_factor`. The evolution length factor therefore sets how far into the wake the last value is reached, as a fraction of the wake length. With the default value of 1.0, the last value is only reached at the end of the wake, while a value of 0.3 means that it is reached after 30% of the wake, and kept constant after that. A lower value therefore gives a faster growth of the viscous core. Note that the core length is set per panel index, so the physical distance where the last value is reached depends on the time step and the velocity, in the same way as the wake length. See the rotor sail [example](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/pystormbird/examples) with two rotors for a demonstration of the effect of this parameter.

#### Computational settings
- `symmetry_condition`: Symmetry plane, as explained [above](#symmetry-condition).
- `far_field_ratio`: Determines when the induced velocity from a panel is calculated with a simplified far field model. If the distance from the center of the panel to the point is larger than the ratio times the largest side length of the panel, the panel is treated as a point doublet, which is faster to compute than the four vortex lines. The far field model does not include the viscous core. The default value is 5.0.
- `neglect_self_induced_velocities`: If true, the induced velocities on a wing from its own wake are neglected, so that only the velocities induced by other wings are included. This is useful if the effect of the self-induced velocities is accounted for in another way, for instance with CFD data, and the only reason for running the lifting line simulation is to calculate the wing-to-wing interaction. **Warning**: this should probably always be used in combination with a [prescribed circulation shape](../line_model/circulation_strength.md) in the line force model, to maintain a realistic circulation distribution.
- `steady_state_strength_update`: If true, all panels in a streamwise row get the same strength as the first panel, rather than having the strength shifted downstream one panel per time step. The whole row is then treated as unknown by the solver, in the same way as a horseshoe vortex in the quasi-steady wake. This gives a steady state solution for the current wake shape, without having to wait for the strength to be convected down the wake. It is useful for steady cases where the dynamic wake is used for its other features, such as a deformed wake shape or a viscous core that grows along the wake, but should not be used for cases where the dynamic variations in the wake strength are of interest.

## Wake export and induced velocities

After a time step, the wake can be used for post-processing, both for the quasi-steady and the dynamic wake. The wake always represents the state at the last completed time step. A quasi-steady wake is empty before the first time step.

### Induced velocities at arbitrary points
The `induced_velocities` function on the simulation calculates the velocity induced by the wake at any set of points, for instance to visualize the flow field around the sails, or to estimate the effect of the sails on other objects:

```python
points = [[50.0, 0.0, 10.0], [100.0, 0.0, 10.0]]

induced_velocities = simulation.induced_velocities(points)
```

The induced velocities are computed with the same vortex elements and viscous core as in the simulation, including the symmetry condition. Note that the freestream velocity is not included.

### Export of the wake geometry
The `wake_as_vtk` function on the simulation returns the wake as a legacy VTK PolyData object, as raw bytes. The data contains the geometry of the wake, together with the `strength` and the `viscous_core_length` of each vortex element as cell data:

- For a **dynamic wake**, each panel is exported as a quadrilateral.
- For a **quasi-steady wake**, each horseshoe vortex is exported as a polygon with six corners, going along the trailing vortices and the bound vortex, and closed with a line between the far ends of the two trailing vortices. That is, the polygon represents the surface spanned by the horseshoe vortex.

The `binary` argument selects between the binary and ASCII variants of the VTK format. The bytes can be read directly into a mesh, for instance with [pyvista](https://pyvista.org/), or written to files that can be opened in, for instance, [ParaView](https://www.paraview.org/). There is no automatic export of the wake during a simulation, so writing files for each time step must be done in the script that runs the simulation:

```python
from pathlib import Path

output_folder = Path("wake_files")
output_folder.mkdir(exist_ok=True)

for time_step_index in range(nr_time_steps):
    result = simulation.do_step(
        time = time_step_index * time_step,
        time_step = time_step,
        freestream_velocity = freestream_velocity
    )

    with open(output_folder / f"wake_{time_step_index}.vtk", "wb") as wake_file:
        wake_file.write(simulation.wake_as_vtk(binary=True))
```

For more details on the implementation, see the [code documentation](https://docs.rs/stormbird/latest/stormbird/).
