# Solver

The job of the lifting line solver is to find the right circulation strength on the wing for the given state, i.e., the freestream velocity and the motion at the current time step. The challenge lies in the dependency between the circulation strength and the *induced velocities*. Changing the strength also changes the  lift-induced velocities from the the potential theory wake, which means that the strength must be *solved for*, not just calculated.

## Libearized solver with visocus correction

TO COME

## Iterative damped iterations

As mentioned in the [lifting line introduction](./lifting_line_intro.md), the solver currently used in Stormbird is inspired by a simple approach outlined in Anderson (2005), chapter 5.4. The basic principle is to start with a first guess of the circulation distribution and then slowly update the values based on iterative calculations of the lift-induced velocities. In short, for every iteration of a lifting line solver, the following is calculated:

1) The lift-induced velocities from the [wake model](wake.md), where the circulation strength from the last iteration (or initial guess, if it is the first iteration) is used as input to the wake model.
2) A new estimation of the [circulation strength on the line force model](./../line_model/circulation_strength.md) with the current estimate of the lift-induced velocities as input.
3) An updated circulation strength for the next iteration which is based on a mix between the current circulation strength and the new estimated value, controlled by a damping factor.

To write step 3 as an equation: The circulation strength at the iteration \\( i \\) is called \\( \Gamma_i \\). The previous circulation strength is called \\( \Gamma_{i-1} \\), and the circulation strength that is calculated using the current estimation of lift-induced velocities is called \\( \Gamma_{i, estimated} \\). With a damping factor labeled \\(d \\), the relationship between these values are as follows:

\\[
    \Gamma_i = \Gamma_{i-1} + d (\Gamma_{i, estimated} - \Gamma_{i-1})
\\]

The benefit of this solver is that it is extremely simple, and it is generally very robust **if** the damping factor is set low enough. More *fancy* solvers may produce quicker results, but can sometimes struggle with instabilities in very non-linear flow conditions. When running unsteady simulations, it is typically not necessary with many iterations for each time step, as the change in the circulation strength is small.

## Residual, damping factor, and convergence testing

The residual is a measure on how close the solver is to the *correct solution*. It is calculated from the difference in the lift coefficient on each line element with the current best guess of the circulation distribution, \\( \Gamma_i \\) from the equations above, and the lift coefficient directly from the [sectional model](./../sectional_models/sectional_models_intro.md) using the current estimated lift-induced velocities. That is, this will also be the same as the lift coefficient calculated with the current estimated circulation distribution, or \\( \Gamma_{i, estimated} \\) from the equations above.

As an equation, the residual, \\(r\\), for a line element is calculated as follows, where \\(C_L(\Gamma) \\) is the lift coefficient based on the induced velocities due to \\( \Gamma \\):

\\[
    r_i = C_L(\Gamma_{i}) - C_L(\Gamma_{i, estimated})
\\]

The value of this residual *should* go towards zero with successive iterations. Since the value is calculated from the lift-coefficients, it is not dependent on the geometrical dimensions or the freestream velocity. The solver will stop if the value of the residual goes below the `residual_tolerance_absolute` value in the `SolverSettings` below.


## Solver settings
The source code below show the available fields for the lifting line solver settings. The same settings are used for both dynamic and quasi-steady simulations. However, there are slight differences in the default values.

```rust
pub struct SolverSettings {
    pub max_iterations_per_time_step: usize,
    pub damping_factor: f64,
    pub residual_tolerance_absolute: f64,
    pub strength_difference_tolerance: f64,
    pub velocity_corrections: VelocityCorrections,
}
```

An explanation of each field is given below:

- `max_iterations_per_time_step`: This parameter control how many iterations that will be performed **per time step**. That is, if a steady simulation is executed, which generally only runs one time step, it is also the max total number of iterations. The solver might stop before the max number of iterations is reached, if the `convergence_test` structure gives a positive test on a converged solution. The default values for this parameter depends on the simulation mode. In a dynamic case, it is 20 (*which might be excessive... should be tested more*). In a quasi-steady case it is 1000 (*definitely excessive most of the time, but the convergence test will generally make the solver stop long before this*)
- `damping_factor`: Determines how fast the circulation distribution should be updated as explained in the *iterative damped iterations* section above. This value must be specified and is set to 0.05 as default when using a steady wake, and 0.1 when using an unsteady wake.

- `residual_tolerance_absolute`: A value used to determine when the solution is converged based on the residual.
- `strength_difference_tolerance`: A value used to determine when the solution is converged based on the maximum difference butene the previous and next estimated circulation strength.
- `velocity_corrections`: An option to add corrections to the estimated velocity, to handle singularities and difficult cases.

## Velocity corrections
To come!

## References
- Anderson, J. D., 2005. Fundamentals of Aerodynamics. Fourth edition. McGraw hill
