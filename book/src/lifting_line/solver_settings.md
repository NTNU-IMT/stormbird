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
- `circulation_viscosity`: This is an experimental parameter and should be **used with care**. It is set to zero by default, and therefore not used. It adds a viscosity term to the estimated circulation distribution, based on the second derivative of the circulation as a function of span location multiplied with this parameter. The idea and implementation is taken from this [pre-print](https://www.researchgate.net/publication/378262301_An_Efficient_3D_Non-Linear_Lifting-Line_Method_with_Correction_for_Post-Stall_Regime). The paper suggest that an artificial viscosity term can in some cases stabilize the results in challenging conditions for the solver. Similar results are found when using teh same approach in Stormbird. However, the parameter requires careful tuning to work properly. Both too low and too high values may cause instabilities. At the moment, Gaussian smoothing is recommended for cases with unstable results.
- `gaussian_smoothing_length`: If this parameter is used, each iteration will apply a [Gaussian smoothing filter](https://en.wikipedia.org/wiki/Kernel_smoother) to the circulation distribution with a length controller by the parameter. The smoothing length is calculated as the value of this parameter, multiplied with the total wing span of each wing in the line force model. That is, if the value is set to 0.01, the smoothing length will be 1% of the span of each wing, independent of the value of the wing span or the number of sections. Early testing indicate that Gaussian smoothing is an effective way to resolve cases with numerical noise. Values of 0.01 is typically enough. The parameter is turned of by default. 
- `convergence_test`: Sub-structure used to determine when a solver step should be stopped. More explanations will come later...
- `print_log`: When set to true, the solver will print information about how many iterations that are executed before a converged solution is found.  