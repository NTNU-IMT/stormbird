# Lifting line simulations
The basics of the lifting line simulations in Stormbird have a lot in common with the classical approaches made by Lanchester (1907) and Prandtl (1918) more than 100 years ago, and which are also often thought in many introduction courses in fluid dynamics and lifting surfaces. That is, the overall concept and equations is the same. The wing geometry is reduced to vortex lines along the span, the lift and circulation on the line elements are estimated from the local velocity and angle of attack based on a two-dimensional sectional model, and the lift-induced velocities due the estimated circulation is calculated based on a potential theory wake model. 

However, the Stormbird implementation also differs from the classical lifting line approach in at least three broad-stroke ways, explained further in the subsections below

## Non-linear solver
In the classical lifting line method, it is possible the make a *simplified* linear equation system to solve for the right circulation for a given free stream velocity. This is based on the assumptions that the lift-induced velocities are small and that there is a linear relationship between the lift and vertical induced velocities on the wing. As a consequence, only a single equations system must be solved for each free stream condition. It is also possible to make a somewhat extended version of such as linearization that can handle some amount of non-linearity's in the lift. The developers of Stormbid have used such an approach in the past, for instance in the methods presented in Kramer et al. (2018). 

However, we have also found that such linearized approaches do not work well when there are very large non-linearity's in the equation system. Sails tend to operate in ways that give large viscous effects on the lift and very large lift-induced velocities. To handle this, Stormbird uses a slower, but also simple and more robust approach based on *dampened iterations*. The solver is largely inspired by the methods described in Anderson (2005), chapter 5.4. This makes it possible to simulate wings operating close to, and above, stall, and the shape of the sectional lift can be arbitrary. 

**Mote details will come**

## Arbitrary shaped wings
The classical methods assumes that both the wing and the wake is completely flat, and that the potential theory vortex wake extends indefinitely far downstream of the wing. These are necessary assumptions to develop an analytical equation system. However, they are not necessary when solving the equations numerically. 

As already explained in the [line model chapter](./../line_model/line_model_intro.md), simulation models are built up of several discrete line elements. This makes it possible to have arbitrary shaped wings, and have multiple wings together in the same simulations. To make this possible in a lifting line simulation, it is necessary with some way to calculate *induced velocities* from a line element. This is done by assuming that each line line element is a *constant strength vortex line*.

Such vortex lines are also often used in panel methods or vortex lattice methods to represent *doublet panels*. See for instance Katz and Plotkin (2001) for more details of such methods. 

The exact formulation for the induced velocity as a function of line geometry and strength are taken from the [VSAERO theory document](https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf), found in the link or in Maskew (1987)

## Unsteady simulations and dynamic wakes

The final extension from the classical lifting line approach is the inclusion of dynamic wakes and unsteady modelling. This means that the wings can move during a simulation, and the velocity input can change as a function of time.

Unsteady simulations comes in two flavours: 1) **quasi-steady** and 2) **dynamic**. In the quasi-steady case, the wake is as it is in a conventional lifting line simulation: It consist of *horseshoe* vortices that extend far downstream from the span lines of each wing for every time step. However, unsteady behavior is still modeled by changes in the *felt velocity* at the line elements due to the motion of the wings or changes in the freestream input. 

In the dynamic case, the wake modeled is extended to consist of many doublet panels, similar to how it would be in an unsteady panel- or vortex lattice method. Both the strength and the shape of the wake panels will vary as a function of time, which allows for proper dynamic modelling of the lift. That is, the lift-induced velocities depend not only on the current *state* of the line model, but also the history of previous states. 

For a single conventional wing, the shape of the vortex wake is typically not that important, which is why is often assumed to be flat in simplified methods. However, we have found that this is not necessarily the case when the lift coefficient becomes very high - such as for rotor sails - or when several sails are placed so close together at the wakes get strongly deformed by other wings. When running dynamic simulations, the shape of the wake can be modified by the induced velocities in the simulation[^note]. This can also be used to simulate steady cases where a detailed wake shape is of interest.

[^note]: It is also possible to turn this of to increase the computational speed. See [wake settings](./wake_settings.md) for more

## References
- Anderson, J. D., 2005. Fundamentals of Aerodynamics. Fourth edition. McGraw hill
- Katz, J., Plotkin, A., 2001. Low-speed aerodynamics. Vol 13. Cambridge university press
- Kramer, J. V., Godø, J. M. K., Steen, S., 2018. Hydrofoil simulations – non-linear lifting line vs CFD. Numerical Towing Tank Symposium, Cortona, Italy
- Lanchester, F. W., 1907. Aerodynamics: Constituting the First Volume of a Complete Work on Aerial Flight
- Maskew, B., 1987. Program VSAERO Theory Document. NASA technical report.
- Prandtl, L., 1918. Tragflügeltheorie. Königliche Gesellschaft der Wissenschaften zu Göttingen.