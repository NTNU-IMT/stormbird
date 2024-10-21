# Python interface
The Python interface to Stormbird is made using a Rust library called [PyO3](https://pyo3.rs/). As a general principle, there is a one-to-one relationship between the functionality available in Python and Rust equivalent functionality. That is, names are kept identical in both languages, and the programming constructs are kept as similar as possible (e.g, data structures in Rust becomes classes in Python, etc.)

However, not all of the Rust library has a Python interface. The plan is generally to only implement what is needed when using the library in a *high-level* setting. 

## Installation

At the moment, the Python interface is distributed as a [wheel file](https://pythonwheels.com/). It can be installed in the usual way using pip, by running the following command in a terminal:

```
pip install [path-to-wheel-file]
```

## Examples
Examples of how to use the Python functionality can be found in the [tutorial chapter](./../tutorials/tutorials.md). 

## Still some JSON input and output
Some of the Python interface still relies on JSON strings as both input and output. This choice is made because it avoids having to update the Python interface when there is change to the Rust code. This is particularly practical when the library is evolving. Generally speaking, JSON strings are used as alternatives to passing in variables directly in initialization functions for classes.

As such, even when using the Python interface to Stormbird, the task is often to create and pass in JSON strings to, for instance, initializer methods to create new objects. This is, however, fairly simple as Python has excellent support for converting dictionaries into JSON strings. 

The JSON-part of the interface might be removed in future versions, when the interface has stabilized, and it is "safe" to implement a direct Python API.

An example of how this works is shown below, where a multi-element foil model for a Stormbird simulation is created from input data that is set up as Python dictionaries, which are then converted to a JSON string:

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

