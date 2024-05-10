# Solver settings
The section below show the available fields in the solver settings for both quasi-steady and dynamic simulations. Every single parameter has a default value, which typically makes sense. In other words, they don't have to be set unless the user wants something different from the default behavior. More explanation of the parameters will come in the future.

## Quasi-steady

```rust
pub struct SteadySolverSettings {
    pub max_iterations: usize,
    pub damping_factor: f64,
    pub include_viscous_wake: bool,
    pub smoothing_length_ratio: Option<f64>,
    pub convergence_test: ConvergenceTest,
    pub print_log: bool,
}
```

## Dynamic
```rust
pub struct UnsteadySolverSettings {
    pub max_iterations_per_time_step: usize,
    pub damping_factor: f64
}
```