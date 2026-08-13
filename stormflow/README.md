# Stormflow

## Purpose
Stormflow is a CFD solver specialized for actuator line simulations. That is, it is NOT intended to be a general solver, but rather dedicated to this one purpose. The hypotheses behind the development is that a simple solver may prove to a be a good fit for this type of modeling.

## Methods
### General CFD details
The solver implements a structured, fixed cell size, cartesian grid and solves the incompressible Navier-Stokes equations using fourth order finite difference schemes. The flow variables are stored using a staggered approach; the pressure lives at the cell center, while the velocity components live on the positive faces of each cell (u on the positive x-face, v on the positive y-face, and w on the positive z-face). Time stepping is done explicitly. The pressure projection step is solved using a geometric multigrid method. The boundaries on the outer grid boundary are enforces through ghost cell, except for the pressure, where the boundaries are also enforces in the kernels directly (to simplify the GPU execution, see the section on Software architecture for more)

### Actuator line method
The actuator line model is based on the Stormbird library. See the [Stormbird folder](../stormbird) for implementation details. In very short terms, the actuator line model computes sectional forces on lifting surfaces, which are projected back to the CFD solver through body forces. 

### Geometry
Other geometries can also be included in the simulation, but then only through deliberately simplified approaches. The goal is not necessarily to predict accurate flow over surfaces in general, but rather capture the overall effect of those geometries on the actuator line model. That is, some simplification on the, e.g., boundary layer and exact details of the flow around the geometry is seen as perfectly fine, as long as the overall effect of that geometry on the flow field is still captured. More specifically, no slip boundaries are implemented using the data immersion technique, while slip boundaries are implemented using a symmetry conditions and conventional immersed boundary ghost cell techniques. The wall boundaries are only enforced on the velocity field directly. 

## Software architecture
The code is deliberately simple, and implements only the necessary functionalities for the purpose of this solver. At the same time, due to the fact that a fixed cell size grid requires many cells - relative to an unstructured grid or structured grid with varying spatial resolution - computational speed is also seen as very important. The main computational bottle neck for this type of simulation is likely the data transfer from the memory to whatever cores executes the code. The different parts of the solver is therefore implemented in as various kernels with a stencil approach for the logic to minimize data transfer. The kernels are generally executed in parallel. On the CPU, the kernels are executed using the rayon crate. Parts of the library can also be executed on the GPU with special GPU kernels written in wgsl, and executed using the wgpu crate. More specifically, at the moment, only the pressure solver is possible to execute on the GPU. The rest of the solver is intended to get a GPU version in the future, except for the actual actuator line model that will remain on the CPU only.
