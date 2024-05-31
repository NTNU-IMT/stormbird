# Solver settings


## Available fields
The section below show the available fields for the lifting line solver settings. The same settings are used for both dynamic and quasi-steady simulations. However, there are slight differences in the default values.

```rust
pub struct SolverSettings {
    pub max_iterations_per_time_step: usize,
    pub damping_factor: f64,
    pub circulation_viscosity: f64,
    pub gaussian_smoothing_length: Option<f64>,
    pub convergence_test: ConvergenceTest,
    pub print_log: bool,
}
```

An explanation of each field is given below:

- `max_iterations_per_time_step`: This parameter control how many iterations that will be performed **per time step**. That is, if a steady simulation is executed, which generally only runs one time step, it is also the max total number of iterations. The solver might stop before the max number of iterations is reached, if the `convergence_test` structure gives a positive test on a converged solution. The default values for this parameter depends on the simulation mode. In a dynamic case, it is 20 (*which might be excessive... should be tested more*). In a quasi-steady case it is 1000 (*definitely excessive most of the time, but the convergence test will generally make the solver stop long before this*)
- `damping_factor`: Determines how fast the circulation distribution should be updated. A new value for the circulation on each line element is estimated for every iteration. The value depends on the induced velocities, which are calculated with the circulation distribution from the previous time step. The update to the circulation for the new time step  is calculated as the `damping_factor` multiplied by the difference in circulation from the previous iteration. The default value for quasi-steady cases is 0.05. The default value for dynamic cases is 0.1.
- `convergence_test`: Sub-structure used to determine when a solver step should be stopped. More explanations will come later...
- `print_log`: When set to true, the solver will print information about how many iterations that are executed before a converged solution is found.  