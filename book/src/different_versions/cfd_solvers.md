# Actuator lines in CFD solvers

Actuator line simulations require a CFD solver to compute the flow field, as explained in the [actuator line chapter](../actuator_line/actuator_line.md). There are currently two ready-made options:

- **[OpenFOAM](https://www.openfoam.com/)** is a general open source CFD solver, widely used both in academia and in industry. The Stormbird interface to OpenFOAM is a volume force model (an `fvOption`) that can be activated together with the standard OpenFOAM solvers, such as `simpleFoam` and `pimpleFoam`. This gives access to the full set of capabilities in OpenFOAM, such as unstructured meshes with local refinement, and a wide range of turbulence models and boundary conditions. See the [OpenFOAM interface chapter](../actuator_line/openfoam_interface.md) for more.
- **Stormflow** is an experimental CFD solver made specifically for actuator line simulations with Stormbird. It uses a uniform cartesian grid, and represents other structures, such as the ship, in a simplified way with an immersed boundary method. This makes the setup simple, as no meshing is needed, and the solver can run on both the CPU and the GPU. The main way to use Stormflow is through its Python interface, `pystormflow`, which is installed with `pip install pystormflow`. See the [Stormflow chapter](../actuator_line/stormflow.md) for more.

Both alternatives can be set up from Python with the `stormbird_setup` library, with as similar input as possible. See the [scripted setup chapter](../actuator_line/scripted_setup.md).

As a general note, the code for both alternatives could also serve as an inspiration for how to connect Stormbird to other CFD solvers.
