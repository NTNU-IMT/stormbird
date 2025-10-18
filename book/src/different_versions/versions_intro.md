# Overview of different versions

Stormbird itself is a [Rust](https://www.rust-lang.org/) library. The Rust programming language was chosen as it combines high computational speed with a modern user-friendly developer experience. One potential way to set up a Stormbird simulation is, therefore, to make a custom Rust executable. However, for those that don't know Rust, or just want to use the library in a high-level setting, it is also possible to use one of the *high-level* interfaces to the core functionality, listed below:

- **The Python interface**, implemented in the `pystormbird` module. This interface is a direct API to the *necessary* Rust functions and data structures for running lifting line simulations. This allows for scripting the setup and execution of simulations using Python. More information about the interface will often be mentioned throughout the rest of the book, as well as a high-level description that can be found [here](python_interface.md). 
- **The [Functional Mockup Interface](https://fmi-standard.org/)**, implemented in the `StormbirdLiftingLine` FMU. This interface is more restricted in what it can do, relative to the Python interface, but serves as a practical way to execute sail simulations together with other FMU-models representing the ship. More information about the FMU-interface can be found [here](fmu_version.md)
- **The OpenFOAM interface** for running actuator line simulations with [OpenFOAM](https://www.openfoam.com/) as the CFD solver. This interface is yet to be described in this book. To come.

