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
The code is deliberately simple, and implements only the necessary functionalities for the purpose of this solver. At the same time, due to the fact that a fixed cell size grid requires many cells - relative to an unstructured grid or structured grid with varying spatial resolution - computational speed is also seen as very important. The main computational bottle neck for this type of simulation is likely the data transfer from the memory to whatever cores executes the code. The different parts of the solver is therefore implemented in as various kernels with a stencil approach for the logic to minimize data transfer. The kernels are generally executed in parallel. On the CPU, the kernels are executed using the rayon crate. The same kernels also exist as GPU kernels written in wgsl, which are executed using the wgpu crate.

### CPU and GPU execution
The solver consists of two main parts: a velocity solver, which updates the velocity field (convection, diffusion, body forces, geometry corrections and boundary conditions), and a pressure solver, which solves the pressure projection step with the multigrid method. Both parts can be executed either on the CPU or on the GPU, and the choice is made independently for each of them. That is, all four combinations are possible. Running both on the GPU is the fastest option, but it is also possible to, for instance, run only the pressure solver on the GPU on a machine where the GPU memory is too limited to hold the entire solver. The velocity solver stores more fields than the pressure solver, and therefore needs roughly three times as much memory.

The platforms are selected in the simulation setup file. The velocity solver is controlled by the `velocity_solver_compute_platform` field, while the pressure solver is controlled by the `compute_platform` field in the multigrid settings. Both default to `CPU`. A setup that runs everything on the GPU looks like this (other fields omitted):

```json
{
  "velocity_solver_compute_platform": "GPU",
  "pressure_solver": {
    "Multigrid": {
      "compute_platform": "GPU"
    }
  }
}
```

The data exchange between the CPU and the GPU is kept to a minimum:

- When both solvers run on the GPU, they share the same GPU device and buffers. The right hand side of the pressure equation is computed directly into the pressure solver's buffer, and the pressure is read directly from the pressure solver's result, so no data is transferred between the CPU and the GPU during the pressure projection. When the two solvers run on different platforms, the right hand side and the pressure are transferred for every pressure solve.
- The actuator line model always runs on the CPU, as it depends on data structures and functionality in the Stormbird library. When the velocity solver runs on the GPU, only the velocity in the cells close to the actuator lines is transferred to the CPU, and only the resulting body forces in the same cells are transferred back.
- The maximum velocity, which is used to compute a time step from a Courant number, is computed on the GPU, so only a single value is transferred for each time step.
- The full fields are only transferred to the CPU when they are explicitly requested, for instance when exporting the results.

All precomputed data, such as the inlet velocity profile, the geometry corrections and the actuator line cells, is computed on the CPU when the simulation is built, and used directly by the CPU version and uploaded once by the GPU version. The CPU and GPU versions therefore implement the same algorithms, and give the same results within floating point accuracy.

### Coarsest level of the multigrid solver
The coarsest level of the multigrid method can either be solved approximately with additional Jacobi iterations (`"coarsest_level_solver": "Jacobi"`, the default), or exactly with Gaussian elimination on a dense matrix (`"coarsest_level_solver": "Exact"`). The exact solver requires the dense matrix to be stored, and is executed on the CPU also for the GPU version of the pressure solver. It can therefore be very slow, and use a lot of memory, if the grid cannot be coarsened much, which is why it is not the default.
