# Lifting line wake

The vortex wakes from the wings are the most important part of a lifting line simulation. They are responsible for modeling how the velocity is affected by the wings themselves. How the velocity should be calculated depends on a several settings variables. Setting up a wake model is therefore done using "wake builders" which both contain settings used directly by the final wake structures and settings used for initializing the wake structures. An overview of the fields available in the wake builders are given in this section.

## Internal wake data
Inside a simualtion structure, the wake is representing as a `WakeData` structure that looks like this:

```rust
pub enum WakeData {
    Dynamic(DynamicWake),
    QuasiSteady(QuasiSteadyWakeSettings),
}
```

That is, the exact nature of the data depends on whether the wake is quasi-steady or dynamic.

In the case of a dynamic case, the wake is represented as many panels where both the strength and position may evolve over time. In the case of the quasi-steady case, the wake is just a set of horseshoe vortices where the trailing vortices are following the freestream. The quasi-steady wake representation is much faster, but also limited in accuracy as it cannot model the full dynamic variations in the lift-induced velocities.

## Quasi-steady wake settings

The quasi-steady wake is constructed on the fly for every time step with the following settings:

```rust
pub struct QuasiSteadyWakeSettings {
    pub wake_length_factor: f64,
    pub symmetry_condition: SymmetryCondition,
    pub viscous_core_length: ViscousCoreLength,
}
```

All of the fields have default settings. The `wake_length_factor` determines how long each trailing vortex should be, as a ratio of the chord length of the wings. It has a default value of 100.0.

The symmetry condition structure specifies if any form of symmetry should be assumed when calculating the lift-induced velocities. The default is no symmetry, but symmetry can also be turned on in x, y, and z direction through setting different values in the Enum.

```rust
pub enum SymmetryCondition {
    #[default]
    NoSymmetry,
    X,
    Y,
    Z,
}
```

The value of the lift-induced velocity will go to infinity, according to potential theory, if one tries to evaluate it too close to the vortex line. This may cause issues in cases where vortex lines from one wing is potentially colliding with another. To handle such problems, Stormbird uses a viscous correction on the lift-induced velocity. When this correction should be applied is determined by a `ViscousCoreLength` structure. The value can be specified either as a value relative to the bound vortex length, or as an absolute value. This is handled by the enum below. `NoViscousCore` turns of the viscous core length. **Tip: turning it off may significantly speed up a simualtion**, but may also cause instabilities if multiple sails are present. The default value is `Relativ(0.1)`, which means that the viscous core length will be 10% of the bound vortex.

```rust
pub enum ViscousCoreLength {
    Relative(f64),
    Absolute(f64),.
    NoViscousCore,
}
```

## Dynamic wake builder
The settings for building a dynamic wake is shown below:

```rust
pub struct DynamicWakeBuilder {
    pub nr_panels_per_line_element: usize,
    pub viscous_core_length: ViscousCoreLength,
    pub viscous_core_length_evolution: ViscousCoreLengthEvolution,
    pub first_panel_relative_length: f64,
    pub last_panel_relative_length: f64,
    pub use_chord_direction: bool,
    pub ratio_of_wake_affected_by_induced_velocities: f64,
    pub far_field_ratio: f64,
    pub shape_damping_factor: f64,
    pub neglect_self_induced_velocities: bool,
    pub initial_relative_wake_length: f64,
    pub write_wake_data_to_file: bool,
    pub wake_files_folder_path: String,
}
```

All values have default values which should make sense in most situations. The most common variable to adjust will be `nr_panels_per_line_element`, the `write_wake_data_to_file`, and the `wake_files_folder_path`. The first determines the number of panels in the streamwise direction. The two second variables are used if you want to export the wake panels to a files for visualizations. If `write_wake_data_to_file` is set to true, the wake panels will be exported as `.vtk` files to the folder defined by the `wake_files_folder_path` string.

For the rest of the variables, see the explanation in the code documentation [LINK TO COME]
