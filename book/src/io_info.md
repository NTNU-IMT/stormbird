# Input/output format

## Overall principle
Stormbird, as any library, consist of many data structures. Some represents the settings for a simulations, such as wake and solver parameters, while others represent input to or result from a simulation. To create and run a simulation it is generally necessary to pass information to the library about the data structures that you whish to create. 

To facilitate simple serialization and deserialization - at least from a coding perspective - Stormbird relies heavily on the [Serde](https://serde.rs/) library, which is "[..] a framework for **ser**ializing and **de**serializing Rust data structures efficiently and generically". In other words, it automates the conversion of data structures to and from some file format. Serde supports many formats, but JSON has been chosen for this case. This means that any input must be passed as JSON strings, and output will be delivered as JSON strings. 

Working with Stormbird is therefor a matter of setting up the right input in a JSON format and then reading and post-processing the output from the resulting JSON format.

Throughout this book there will often be examples of data structures shown as Rust code. This is generally to show the available fields in a structure, to give an impression of which variables it is possible to set. A simple example of how a generic Rust structure is converted to a JSON string from Serde is shown below. 

The Rust version first:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
```

Then the corresponding JSON version with some input data

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

## Default values
Default values are often given for structures representing settings or models. This means that is often not necessary to specify every field in a structure in the input. For instance, in the example above, the `"wake"` structure only has one specified variable. However, the complete wake structure has 11 fields, where some are other structures with more sub fields. The reason only the `"ratio_of_wake_affected_by_induced_velocities"` is given above is that this was the only setting where a different value than the default was wanted.

The goal is to implement reasonable default values on as many variables as possible.

## Python interface
The Python interface to Stormbird is made using a Rust library called [PyO3](https://pyo3.rs/). Although it is possible (and not that difficult) to create interfaces that look and feel like Python code, much of the Python interface still relies on JSON strings as both input and output. This choice is made because it avoids having to update the Python interface when there is change to the Rust code. This is particularly practical when the library is evolving. 

As such, even when using the Python interface to Stomrbird, the task is generally to create and pass in JSON strings. This is, however, not a big problem as Python has excellent support for converting dictionaries into JSON strings. 

An example of how to create a multi-element foil model for a Stormbird simulation in Python is seen below:

```python
import numpy as np
import json
from pystormbird.section_models import VaryingFoil

# Parameters for the model, representing the foil forces at different lap angles
flap_angles = np.radians([0, 5, 10, 15])

cl_zero_angle = np.array([0.0, 0.3454, 0.7450, 1.0352])
mean_stall_angle = np.radians([20.0, 19.0 , 17.8, 16.5])

cd_zero_angle = np.array([0.0101, 0.0154, 0.0328, 0.0542])
cd_second_order_factor = np.array([0.6, 0.9, 1.2, 1.5])

# Loop over the parameters to create individual foil models
foils_data = []
for i_flap in range(len(flap_angles)):
    foils_data.append(
        {
            "cl_zero_angle": cl_zero_angle[i_flap],
            "cd_zero_angle": cd_zero_angle[i_flap],
            "cd_second_order_factor": cd_second_order_factor[i_flap],
            "mean_stall_angle": mean_stall_angle[i_flap]
        }
    )

# Collect the foil models into a "varying foil" model
foil_dict = {}

foil_dict["internal_state_data"] = flap_angles.tolist()
foil_dict["foils_data"] = foils_data

# Generate a JSON input string
input_str = json.dumps(foil_dict)

# Pass it to the Stormbird library
foil_model = VaryingFoil(input_string)
```

