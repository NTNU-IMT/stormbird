# Stormflow

Stormflow is a simple CFD solver specialized for actuator line simulations. That is, it is NOT intended to compete with more general solvers, like OpenFOAM. Rather, it implements the bare necessities for running actuator line simulations using very simple principles. The benefit of this solver over alternatives are not yet clear. It is mostly an experiment to investigate whether this simple solver type make sense for this application.

## "Features"
- Structured orthogonal grid (-> no spatial varying resolution!)
- Explicit time stepping (-> need for very small time steps!)
- No solid walls (yet)
- No turbulence models (yet)
-
