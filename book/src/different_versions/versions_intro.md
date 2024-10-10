# Overview of different versions

Stormbird itself is a [Rust](https://www.rust-lang.org/) library. The Rust programming language was chosen as it combines of high computational speed with a modern user friendly developer experience. One potential way to set up a Stormbird simulation is to make a custom Rust executable. However, this is generally not recommended, unless you are anyhow making a Rust application that need some sail-simulation capabilities. In stead, it is recommended to use one of the three *high-level* interfaces to the core functionality, listed below:

- **The Python interface**, implemented in the `pystormbird` module. This interface is a direct API to the *necessary* Rust functions and data structures for running lifting line simulations. This allows for scripting the setup and execution of simulations using Python. It is therefore considered the most general user interface. More information about the Python interface can be found [here](python_interface.md), and the Python interface will also often be referenced in the rest of the book, along with the Rust code directly. 
- **The [Functional Mockup Interface](https://fmi-standard.org/)**, implemented in the `StormbirdLiftingLine` `FMU`. This interface is much more restricted in what it can do, relative to the Python interface, but serves as a practical way to execute sail simulations together with other FMU-models representing the ship. More information about the FMU-interface can be found [here](fmu_version.md)
- **The OpenFOAM interface** for running actuator line simulations with [OpenFOAM](https://www.openfoam.com/) as the CFD solver. This interface is yet to be described in this book. To come.

## Input and output - overall principle
Stormbird, as any library, consist of many data structures. Some represents the settings for a simulations, such as wake and solver parameters, while others represent input to or result from a simulation. To create and run a simulation it is generally necessary to pass information to the library about the data structures that you whish to create. This is the case for all the interfaces listed above. 

To facilitate simple serialization and deserialization - at least from a coding perspective - Stormbird relies heavily on the [Serde](https://serde.rs/) library, which is "[..] a framework for **ser**ializing and **de**serializing Rust data structures efficiently and generically". In other words, it is a library to automate the conversion of data structures to and from different file formats. Serde supports many formats, but JSON has been chosen for this case. This means that any input must be passed as JSON strings, and output will often be delivered as JSON strings. 

Working with Stormbird is therefor often a matter of setting up the right input in a JSON format and then reading and post-processing the output from the resulting JSON format.

Throughout this book there will often be examples of data structures shown as Rust code. This is generally to show the available fields in a structure, to give an impression of which variables it is possible to set. All of these Rust-structures has a corresponding JSON representation. A simple example of how a generic Rust structure is converted to a JSON string from Serde is shown below. 

An example of a Rust struct first:

```rust
pub struct SpatialVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
```

Then the corresponding JSON version with the input data

```json
{
    "x": 1.0,
    "y": 0.0,
    "z": 1.2
}
```


An example of a complete input string to Stormbird can be seen below. More explanations about this input will come later:

```json
{
    "line_force_model": {
        "wing_builders": [
            {
                "section_points": [
                    {"x": 125.0, "y": 0.0, "z":-20.0},
                    {"x": 125.0, "y": 0.0, "z":-60.0}
                ],
                "chord_vectors": [
                    {"x": -8.0, "y": 0.0, "z": 0.0},
                    {"x": -8.0, "y": 0.0, "z": 0.0}
                ],
                "section_model": {
                    "Foil": {}
                }
            },
            {
                "section_points": [
                    {"x": 45.0, "y": 0.0, "z": -20.0},
                    {"x": 45.0, "y": 0.0, "z": -60.0}
                ],
                "chord_vectors": [
                    {"x": -8.0, "y": 0.0, "z": 0.0},
                    {"x": -8.0, "y": 0.0, "z": 0.0}
                ],
                "section_model": {
                    "Foil": {}
                }
            }
        ],
        "nr_sections": 10
    },
    "simulation_mode": {
        "Dynamic": {
            "wake": {
                "ratio_of_wake_affected_by_induced_velocities": 0.25
            },
            "solver": {
                "damping_factor": 0.2,
                "max_iterations_per_time_step": 3
            }
        }
    },
    "write_wake_data_to_file": true,
    "wake_files_folder_path": "output/wake_files"
}
```

### Default values
Default values are often given for structures representing settings or models. This means that is often not necessary to specify every field in a structure in the input. For instance, in the example above, the `"wake"` structure only has one specified variable. However, the complete wake structure has 12 fields, where some are other structures with more sub fields. The reason only the `"ratio_of_wake_affected_by_induced_velocities"` is given above is that this was the only setting where a different value than the default was wanted.

The goal is to implement reasonable default values on as many variables as possible.