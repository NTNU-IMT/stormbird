# Lifting line Wake

The vortex wakes from the wings are the most important part of a lifting line simulation. They are responsible for modelling how the velocity is affected by the wings themselves. How the velocity should be calculated depends on a several settings variables. Setting up a wake model is therefore done using "wake builders" which both contain settings used directly by the final wake structures and settings used for initializing the wake structures. An overview of the fields available in the wake builders are given in this section. **More detailed information will come later**. 

## Common settings structures
This section gives an overview of sub-structures used in the setup of wake structures. They are relevant for both quasi-steady and dynamic simulations.

```rust
pub enum SymmetryCondition {
    NoSymmetry,
    X,
    Y,
    Z,
}
```

```rust
pub enum ViscousCoreLength {
    Relative(f64),
    Absolute(f64),
}
```

## Quasi-steady

```rust
pub struct SteadyWakeBuilder {
    pub wake_length_factor: f64,
    pub symmetry_condition: SymmetryCondition,
    pub viscous_core_length: ViscousCoreLength,
}
```

## Dynamic
Main wake builder structure:

```rust
pub struct UnsteadyWakeBuilder {
    pub wake_length: WakeLength,
    pub viscous_core_length: ViscousCoreLength,
    pub viscous_core_length_end: Option<ViscousCoreLength>,
    pub first_panel_relative_length: f64,
    pub last_panel_relative_length: f64,
    pub use_chord_direction: bool,
    pub strength_damping_factor: f64,
    pub strength_damping_factor_separated: Option<f64>,
    pub symmetry_condition: SymmetryCondition,
    pub ratio_of_wake_affected_by_induced_velocities: f64,
    pub far_field_ratio: f64,
    pub shape_damping_factor: f64,
    pub neglect_self_induced_velocities: bool
}
```

Special sub structures used only for the dynamic wake:

```rust
pub enum WakeLength {
    NrPanels(usize),
    TargetLengthFactor(f64),
}
```