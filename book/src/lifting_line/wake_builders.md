# Wake builders

The vortex wakes from the wings are the most important part of a lifting line simulation. They are responsible for modelling how the velocity is affected by the wings themselves. How the velocity should be calculated depends on a several settings variables. Setting up a wake model is therefore done using "wake builders" which both contain settings used directly by the final wake structures and settings used for initializing the wake structures. An overview of the fields available in the wake builders are given in this section. **More detailed information will come later**. 

## Common settings structures
This section gives an overview of sub-structures used in the setup of wake structures. They are relevant for both quasi-steady and dynamic simulations.

```rust
pub struct VelocityCorrectionsBuilder {
    pub max_magnitude_ratio: Option<f64>,
    pub correction_factor: Option<f64>,
}
```

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
    pub induced_velocity_corrections: VelocityCorrectionsBuilder
}
```

## Dynamic
Main wake builder structure:

```rust
pub struct UnsteadyWakeBuilder {
    pub wake_length: WakeLength,
    pub viscous_core_length: ViscousCoreLength,
    pub first_panel_behavior: FirstPanelBehavior,
    pub strength_damping_last_panel_ratio: f64,
    pub symmetry_condition: SymmetryCondition,
    pub ratio_of_wake_affected_by_induced_velocities: Option<f64>,
    pub far_field_ratio: f64,
    pub shape_damping_factor: f64,
    pub induced_velocity_corrections: VelocityCorrectionsBuilder,
    pub viscous_core_length_off_body: Option<ViscousCoreLength>,
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

```rust
pub enum FirstPanelBehavior {
    ChordFixed(f64),
    VelocityFixed(f64)
}
```