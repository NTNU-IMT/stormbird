# pystormflow

A Python interface to Stormflow, a CFD solver specialized for actuator line simulations of lifting surfaces with the Stormbird library.

## Requirements

> **⚠️ Rust Toolchain Required**
>
> This package is distributed as source only and must be compiled during installation.
> You **must** have the Rust toolchain installed before installing this package.
>
> **Install Rust:** Visit [https://rust-lang.org/](https://rust-lang.org/)

Stormflow can run parts of the solver on the GPU through [wgpu](https://wgpu.rs/), which requires up-to-date graphics drivers with support for Vulkan, Metal or DirectX 12.

## Installation

Once Rust is installed, install pystormflow with pip:

```bash
pip install pystormflow
```

Or install from source:

```bash
git clone https://github.com/NTNU-IMT/stormbird.git
cd stormbird/interfaces/pystormflow
pip install .
```

## Use Cases

- Running actuator line simulations with Stormflow through Python scripting
- Post-processing of flow fields and sectional forces directly in Python

## Implementation Details

The interface is made using [PyO3](https://pyo3.rs/). A simulation is set up from a JSON string, which can be created using Python's built-in `json` module for dictionaries, or using the [stormbird_setup](https://github.com/NTNU-IMT/stormbird/tree/main/interfaces/stormbird_setup) helper package. See the [Stormflow README](https://github.com/NTNU-IMT/stormbird/tree/main/stormflow) for a description of the solver and its settings.

```python
from pystormflow import Simulation

with open("setup.json") as f:
    simulation = Simulation(f.read())

simulation.do_steps_until_end_time(end_time=10.0, courant_number=0.5)

simulation.export_fields_as_vtk("fields.vtk", binary=True)
result = simulation.get_stormbird_simulation_result()
```

## Development

For a test build using cargo:
```bash
cargo build
```

To build a distributable source package:
```bash
maturin sdist
```
