# Solver

The job of the lifting line solver is to find the right circulation strength on the wing for the given state, i.e., the freestream velocity and the motion at the current time step. The challenge lies in the dependency between the circulation strength and the *induced velocities*. Changing the strength also changes the lift-induced velocities from the potential theory wake, which means that the strength must be *solved for*, not just calculated.

Two solvers currently exist: a linearized solver with viscous corrections and a full non-linear solver based on dampened iterations.

## Linearized solver with a simple viscous correction

The linearized solver creates an equation system like the original lifting line method. The lift-induced velocities are assumed to only affect the angle of attack and the lift as a function of angle of attack is assumed to be linear. More in depth explanations may be found in text books like Anderson (2005).

The result of applying the normal lifting line assumptions is a linear equation system that can be solved using a conventional linear algebra solver. The linearized solver therefore works by first setting up the equation system as a matrix and a right-hand side vector, before solving it using conventional Gaussian elimination. Control points without any velocity get zero circulation, as the equations are otherwise singular.

However, the **procedure above is only the first step**. Due to the assumption of linear lift as a function of angle of attack, the resulting circulation that is returned from the solver is without any stall- or other non-linear effects on the lift. To account for this, a simplified viscous correction method is applied after solving for the circulation strength using a linear solver. It consists of the following steps:

1) Calculate the lift-induced velocities and resulting effective angle of attack with the solved circulation strength
2) Calculate the lift both with a linearized sectional model and the full sectional model, including stall effects
3) Correct the solved circulation strength by multiplying it with the full lift and dividing it by the linearized lift

The forces are then calculated with the corrected circulation strength and the effective angles of attack from step 1. The post-stall lift, which is added to the lift from the circulation in the [force calculations](../line_model/force_calculations.md), is therefore based on the same angles of attack as the correction of the circulation.

An optional fourth step, controlled by the `induced_velocity_correction_method` setting below, recalculates the lift-induced velocities and effective angles of attack from the corrected circulation strength before the forces are calculated. This was the default in earlier versions of Stormbird. However, the circulation is then corrected with different angles of attack than the forces are calculated with. Right before stall, a reduced circulation gives lower lift-induced velocities and therefore a larger angle of attack in the force calculations, so that sections can get both an almost unstalled circulation and post-stall lift. This shows up as a jump in the lift right before stall, and the step is therefore not used by default.

This solver is found to work fine for **quasi-steady** cases, but does also tend to predict **stall at a larger angle of attack** than the full non-linear solver below. However, the stall-issue can be handled by tuning the stall behavior of the sectional model to 3D data of a single sail. For quasi-steady cases it will be significantly faster than running the full non-linear solver described in the next section, and is therefore set to the default solver for such cases.

## Non-linear solver using damped iterations

The second solver is inspired by a simple approach outlined in [Anderson (2005), chapter 5.4](../literature/simulation_methods.md#fundamentals-of-aerodynamics-2005). The basic principle is to start with a first guess of the circulation distribution and then slowly update the values based on iterative calculations of the lift-induced velocities. In short, for every iteration of a lifting line solver, the following is calculated:

1) The lift-induced velocities from the [wake model](wake.md), where the circulation strength from the last iteration (or initial guess, if it is the first iteration) is used as input to the wake model.
2) A new estimation of the [circulation strength on the line force model](./../line_model/circulation_strength.md) with the current estimate of the lift-induced velocities as input.
3) An updated circulation strength for the next iteration which is based on a mix between the current circulation strength and the new estimated value, controlled by a damping factor.

To write step 3 as an equation: The circulation strength at the iteration \\( i \\) is called \\( \Gamma\_i \\). The previous circulation strength is called \\( \Gamma\_{i-1} \\), and the circulation strength that is calculated using the current estimation of lift-induced velocities is called \\( \Gamma\_{i, estimated} \\). With a damping factor labeled \\(d \\), the relationship between these values are as follows:

\\[
    \Gamma\_i = \Gamma\_{i-1} + d (\Gamma\_{i, estimated} - \Gamma\_{i-1})
\\]

The benefit of this solver is that it is simple and technically more correct than the linearized solver, as there are no assumptions about small lift-induced velocities. It will also handle non-linear effects on the lift directly, without any post-solver corrections, like in the case for the linearized solver. It is generally robust **if the damping factor is set low enough**, but may also give noise in the final results right at the stall point in some cases. This is typically handled by applying some [smoothing to the circulation strength](../line_model/circulation_strength.md) in the line force model. It is also the **most suitable solver for unsteady simulations**, which typically do not require many iterations for each time step, as the change in the circulation strength is small.

## Residual, damping factor, and convergence testing

The residual is a measure on how close the solver is to the *correct solution*. It is calculated from the difference in the lift coefficient on each line element with the current best guess of the circulation distribution, \\( \Gamma\_i \\) from the equations above, and the lift coefficient directly from the [sectional model](./../sectional_models/sectional_models_intro.md) using the current estimated lift-induced velocities. That is, this will also be the same as the lift coefficient calculated with the current estimated circulation distribution, or \\( \Gamma\_{i, estimated} \\) from the equations above. The lift coefficient from the sectional model is the same part of the lift that is used to compute the circulation strength, i.e., the pre-stall lift by default, and the sum of the pre-stall and post-stall lift if the `include_viscous_lift_in_the_circulation` setting is used (see the [circulation strength chapter](../line_model/circulation_strength.md)).

As an equation, the residual, \\(r\\), for a line element is calculated as follows, where \\(C\_L(\Gamma) \\) is the lift coefficient based on the induced velocities due to \\( \Gamma \\):

\\[
    r\_i = C\_L(\Gamma\_{i}) - C\_L(\Gamma\_{i, estimated})
\\]

The value of this residual *should* go towards zero with successive iterations. Since the value is calculated from the lift-coefficients, it is not dependent on the geometrical dimensions or the freestream velocity. The solver will stop if the average absolute value of the residual over all line elements goes below the `residual_tolerance_absolute` value in the solver settings below. The solver will also stop if the maximum difference between the new estimate of the circulation strength and the current value, before the damping is applied, goes below the `strength_difference_tolerance` value.


## Solver settings
The source code below shows the available fields for the lifting line solver settings. For quasi-steady cases, a **builder** is used to set the right settings for this application area

```rust
pub enum Solver {
    Iterative(Iterative),
    Linearized(Linearized)
}

pub enum QuasiSteadySolverBuilder {
    Iterative(QuasiSteadyIterativeBuilder),
    Linearized(Linearized)
}
```

### Linearized settings

For the linearized settings, the following source code shows the available fields:

```rust
pub struct Linearized {
    pub induced_velocities: InducedVelocitySettings,
    pub disable_viscous_corrections: bool,
    pub induced_velocity_correction_method: InducedVelocityCorrectionMethod
}

pub enum InducedVelocityCorrectionMethod {
    NoCorrection,   // Default
    FullCorrection,
}
```

The only one that could be interesting to modify is the `induced_velocities`, which are explained in [their own section below](#induced-velocities-and-velocity-corrections). The other two are mainly for testing purposes and not necessary to adjust for normal use cases. The `induced_velocity_correction_method` selects whether the lift-induced velocities are recalculated after the viscous correction (`FullCorrection`) or not (`NoCorrection`), as explained above.

### Non-linear settings

For the non-linear settings, the following source code shows the available fields:

```rust
pub struct Iterative {
    pub max_iterations_per_time_step: usize,
    pub damping_factor: f64,
    pub residual_tolerance_absolute: f64,
    pub strength_difference_tolerance: f64,
    pub induced_velocities: InducedVelocitySettings,
    pub use_raw_circulation_during_iterations: bool,
    pub start_with_linearized_solution: bool,
    pub correction_factor_initial_solution: f64,
    pub variable_to_update: VariableToUpdate
}

pub enum VariableToUpdate {
    CirculationStrength,
    AngleOfAttack
}
```

The `QuasiSteadyIterativeBuilder` used when building a solver for quasi-steady cases is in general a structure with the same fields, except for the `use_raw_circulation_during_iterations` field, but with different default settings that are more suitable for steady cases.

An explanation of each field is given below:

- `max_iterations_per_time_step`: This parameter controls how many iterations that will be performed **per time step**. That is, if a steady simulation is executed, which generally only runs one time step, it is also the max total number of iterations. The solver might stop before the max number of iterations is reached, if the convergence test based on the `residual_tolerance_absolute` or the `strength_difference_tolerance` gives a positive test on a converged solution. The default values for this parameter depend on the simulation mode. In a dynamic case, it is 40 (*which might be excessive... should be tested more*). In a quasi-steady case it is 1000 (*definitely excessive most of the time, but the convergence test will generally make the solver stop long before this*)
- `damping_factor`: Determines how fast the circulation distribution should be updated as explained in the *iterative damped iterations* section above. It is set to 0.04 as default when using a quasi-steady wake, and 0.05 when using a dynamic wake.
- `residual_tolerance_absolute`: A value used to determine when the solution is converged based on the residual. The default value is 1e-4.
- `strength_difference_tolerance`: A value used to determine when the solution is converged based on the maximum difference between the previous and next estimated circulation strength. The default value is 1e-6.
- `induced_velocities`: Settings for how the lift-induced velocities are used, including options to neglect the self-induced velocities and to add corrections to handle singularities and difficult cases. See the [section below](#induced-velocities-and-velocity-corrections).
- `use_raw_circulation_during_iterations`: A boolean that can be set to true if the [circulation corrections](../line_model/circulation_strength.md) in the line force model should be skipped when estimating the circulation strength during the iterations. The default is false. This option is only available for dynamic simulations.
- `start_with_linearized_solution`: A boolean that can be set to true if you want the first iteration to estimate the circulation distribution using a linear solver. The rest of the iterations will then use the normal non-linear iterations to update from the linearized solver.
- `correction_factor_initial_solution`: A factor that the initial circulation distribution, i.e., the solution from the previous time step, is multiplied with before the first iteration. The default value is 1.0, which means no correction.
- `variable_to_update`: Determines which variable the damping factor is applied to in each iteration. The default is `CirculationStrength`, which corresponds to the equation above. If set to `AngleOfAttack`, the damping is instead applied to the update of the angle of attack on each line element, and the circulation strength is then calculated directly from the damped angles of attack. Note that the convergence test does not work with this option at the moment, so the solver always runs `max_iterations_per_time_step` iterations.

## Induced velocities and velocity corrections
The lift-induced velocities at the control points are always computed in two parts:

- **Self-induced** velocities: the velocities induced on a wing by its own wake.
- **Other-induced** velocities: the velocities induced on a wing by the wakes of all the other wings.

For a single wing, all the induced velocities are self-induced. The two parts are computed together, in the same loops, so the split does not make the simulations slower. It makes it possible to treat the two parts differently, which is controlled by the `induced_velocities` field in the solver settings:

```rust
pub struct InducedVelocitySettings {
    pub neglect_self_induced_velocities: bool,                     // Default: false
    pub self_induced_correction: SelfInducedVelocityCorrection,    // Default: NoCorrection
    pub total_correction: TotalVelocityCorrection,                 // Default: NoCorrection
}

pub enum SelfInducedVelocityCorrection {
    NoCorrection,
    MaxInducedVelocityMagnitudeRatio(f64),
}

pub enum TotalVelocityCorrection {
    NoCorrection,
    MaxInducedVelocityMagnitudeRatio(f64),
    FixedMagnitudeEqualToFreestream,
}
```

The velocity at each control point is computed in the following order:

1. The self-induced velocity is set to zero if `neglect_self_induced_velocities` is true, and is otherwise corrected with the `self_induced_correction`.
2. The induced velocity is the sum of the self-induced velocity from step 1 and the other-induced velocity.
3. The `total_correction` is applied, together with the freestream velocity.

### Neglecting the self-induced velocities
If `neglect_self_induced_velocities` is true, only the velocities induced by other wings are included. This is useful if the effect of the self-induced velocities is accounted for in another way, for instance with CFD data, and the only reason for running the lifting line simulation is to calculate the wing-to-wing interaction. It works for both quasi-steady and dynamic simulations. **Warning**: this should probably always be used in combination with a [prescribed circulation shape](../line_model/circulation_strength.md) in the line force model, to maintain a realistic circulation distribution.

### Velocity corrections
Velocity corrections are special models that can be used to alter the resulting lift-induced velocities computed from the circulation distributions in the solvers. The purpose is two-fold. For one, applying corrections to the lift-induced velocities may stabilize the solver. Second, the velocity corrections may be used to correct for physical effects that are not directly part of the line force model such as end-disks. The drag on rotor sails, in particular, may be estimated to be too high compared to values estimated with high-fidelity CFD simulations without some corrections applied to the lift-induced velocities, which is likely due to the presence of the large end-disks on such sails.

The default is to use no corrections, so that the lift-induced velocities from the solver are calculated based on the raw circulation strength. The available corrections are:

- `MaxInducedVelocityMagnitudeRatio` makes sure the magnitude of the lift-induced velocity never exceeds a ratio of the freestream velocity. The ratio is supplied as an input. A typical value could be to set the ratio to 1.0, which would be the same as saying that the lift-induced velocity should never exceed the freestream magnitude. As a `self_induced_correction`, only the self-induced velocity is limited, while the velocities induced by other wings are kept as they are. As a `total_correction`, the sum of the self-induced and the other-induced velocities is limited.
- `FixedMagnitudeEqualToFreestream` is only available as a `total_correction`. It computes a velocity vector based on the raw lift-induced velocity and the freestream that is limited in magnitude to the freestream velocity but allowed to rotate freely. That is, this correction allows the lift-induced velocity to change the orientation of the effective velocity at each line segment, but not the magnitude. This can, for instance, be used to force the non-linear solver to behave more like a linear solver.

The two corrections can be combined, for instance to limit the self-induced velocity close to the ends of a wing with a large circulation at the ends, such as a rotor sail, without limiting the interaction with other wings.
