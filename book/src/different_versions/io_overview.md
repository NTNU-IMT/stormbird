# General about input and output

Stormbird, as any library, consist of many data structures. Some represents the settings for a simulations, such as wake and solver parameters, while others represent input to or result from a simulation. To create and run a simulation it is generally necessary to pass information to the library about the data structures that you whish to create. This is the case for all the interfaces. 

To facilitate simple serialization and deserialization - at least from a coding perspective - Stormbird relies heavily on the [Serde](https://serde.rs/) library, which is "[..] a framework for **ser**ializing and **de**serializing Rust data structures efficiently and generically". In other words, it is a library to automate the conversion of data structures to and from different file formats. Serde supports many formats, but JSON has been chosen for Stormbird. This means that any input must be passed as JSON strings, and output will often be delivered as JSON strings. 

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

## Default values
Default values are often given for structures representing settings or models. This means that is often not necessary to specify every field in a structure in the input. For instance, in the example above, the `"wake"` structure only has one specified variable. However, the complete wake structure has 12 fields, where some are other structures with more sub fields. The reason only the `"ratio_of_wake_affected_by_induced_velocities"` is given above is that this was the only setting where a different value than the default was wanted.

The goal is to implement reasonable default values on as many variables as possible.