# Lifting line simulations
The basics of the lifting line simulations in Stormbird have a lot in common with the classical approaches made by Lanchester (1907) and Prandtl (1918) more than 100 years ago. That is, the overall concept and equations is the same. For instance, the lift and circulation on the line elements are estimated from the local velocity and angle of attack based on a two-dimensional sectional model and the lift-induced velocities due the estimated circulation is calculated based on a potential theory wake model. 

However, the Stormbird implementation also differs from the classical lifting line approach in at least three broad-stroke ways, explained further in the subsections below

## Non-linear solver
In the classical lifting line method, it is possible the make a *simplified* linear equation system to solve for the right circulation for a given free stream velocity. This is based on the assumptions that the lift-induced velocities are small and that there is a linear relationship between the lift and vertical induced velocities on the wing. As a consequence, only a single equations system must be solved for each free stream condition. It is also possible to make a somewhat extended version of such as linearization that can handle some amount of non-linearity's in the lift. The developers of Stormbid have used such an approach in the past, for instance in the methods presented in Kramer et al. (2018). 

However, we have also found that such linearized approaches do not work well when there are very large non-linearity's in the equation system. Sails tend to operate in ways that give large viscous effects on the lift and very large lift-induced velocities. To handle this, Stormbird uses a slower but simple and more robust approach based on dampened iterations, largely inspired by the methods described in Anderson (2005), chapter 5.4.

This makes it possible to simulate wings operating close to, and above, stall, and the shape of the sectional lift can be arbitrary.

## Arbitrary shaped wings
The classical methods assumes that both the wing and the wake is complexly flat, and that the wake has the same strength all

## Unsteady simulations

## References
- Anderson, J. D., 2005. Fundamentals of Aerodynamics, fourth edition. 
- Kramer, J. V., Godø, J. M. K., Steen, S., 2018. Hydrofoil simulations – non-linear lifting line vs CFD. Numerical Towing Tank Symposium, Cortona, Italy
- Lanchester, F. W., 1907. Aerodynamics: Constituting the First Volume of a Complete Work on Aerial Flight
- Prandtl, L., 1918. Tragflügeltheorie. Königliche Gesellschaft der Wissenschaften zu Göttingen.