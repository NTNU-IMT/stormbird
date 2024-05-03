# Input/output format

Stormbird, as any library, consist of many data structures. Some represents the methods themselves, such as a lifting line simulation, or parts of a method, such as a wake structure. Others represent input to or result from a simulation. To create a simulation it is generally necessary to pass information to the library about the data structures that you whish to create. 

To facilitate simple serialization and deserialization - at least from a coding perspective - Stormbird relies heavily on the [Serde](https://serde.rs/) crate ( "crate" = a library in rust), which is "[..] is a framework for **ser**ializing and **de**serializing Rust data structures efficiently and generically". Furthermore, although Serde supports many data formats, the `JSON` format has been chosen for this library. This means that any input must be passed as `JSON` strings, and any output must be post-processed as `JSON` strings. 

Working with Stormbird is therefor a matter of setting up the right input in a `JSON` format and then reading and post-processing the output from the resulting `JSON` format.

An example of an input string to Stormbird can be seen below. More explanations about this input will come later:

```json
{
    "line_force_model": {
        "wing_builders": [
            {
                "section_points": [
                    [125.0, 0.0, -20.0],
                    [125.0, 0.0, -60.0]
                ],
                "chord_vectors": [
                    [-8.0, 0.0, 0.0],
                    [-8.0, 0.0, 0.0]
                ],
                "section_model": {
                    "Foil": {}
                }
            },
            {
                "section_points": [
                    [45.0, 0.0, -20.0],
                    [45.0, 0.0, -60.0]
                ],
                "chord_vectors": [
                    [-8.0, 0.0, 0.0],
                    [-8.0, 0.0, 0.0]
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
                "ratio_of_wake_affected_by_induced_velocities": 0.0
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

## Python interface
Although it would be possible (and not that difficult) to create other types of interfaces to the library, for instance by making settings available directly as `Python` classes, it would also increase the maintenance in terms of keeping the interface up to date with any changes to the underlying Rust-library. This manual task is avoided by using Serde which is practical from a developer perspective, especially while the library is evolving. 

This means that even the Python interface to Stormbird relies heavily on `JSON` strings. Luckily, `Python` has excellent support for `JSON` strings in the standard library. For now, see the examples for more on this.

