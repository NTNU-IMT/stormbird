# Velocity input

At the moment, the Python version of Stormbird can only handle freestream velocity as a single vector. That is, the velocity is specified in terms of its x, y, and z components. However, on the Rust side it is also possible to specify spatially varying velocities due to a model of the atmospheric boundary layers and varying velocity from other sources - such as CFD simulations of hull and superstructure. 

A high priority going forward is to get this functionality into the Python and FMU versions of Stormbird as well. This section will be updated when that happens. Stay tuned!