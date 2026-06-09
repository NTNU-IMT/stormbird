# Stormflow

Stormflow is a simple CFD solver specialized for actuator line simulations. That is, it is NOT intended to compete with more general solvers, like OpenFOAM. Rather, it implements the bare necessities for running actuator line simulations using very simple principles. The benefit of this solver over alternatives are not yet clear. It is mostly an experiment to investigate whether this simple solver type make sense for this application.

## "Features"
The choice of methods and features to implement are made such that the code becomes as straight forward as possible. This means, among other things, the following:
- Structured cartesian grid with uniform cell size -> no spatial varying resolution!
- Explicit time stepping -> need for very small time steps!
- Solid walls through an immersed boundary method
- No turbulence models
