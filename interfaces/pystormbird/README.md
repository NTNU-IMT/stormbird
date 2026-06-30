# pystormbird

A Python interface to the Stormbird library for simulating lifting surfaces.

## Requirements

> **⚠️ Rust Toolchain Required**
>
> This package is distributed as source only and must be compiled during installation.
> You **must** have the Rust toolchain installed before installing this package.
>
> **Install Rust:** Visit [https://rust-lang.org/](https://rust-lang.org/)

## Installation

Once Rust is installed, install pystormbird with pip:

```bash
pip install pystormbird
```

Or install from source:

```bash
git clone https://github.com/NTNU-IMT/stormbird.git
cd stormbird/interfaces/pystormbird
pip install .
```

## Use Cases

- Running lifting line simulations through Python scripting
- Testing parts of the code that require plotting and visual inspection

## Implementation Details

The interface is made using [PyO3](https://pyo3.rs/). The layout and structure of the code follow the source code in Stormbird as much as possible.

The initialization of most classes is done using JSON strings, to avoid unnecessary maintenance of code. You can use Python's built-in `json` module for dictionaries, or use the [stormbird_setup](../stormbird_setup/) helper package.

It is not a goal to offer an interface to every aspect of the Stormbird library. Rather, an interface is only made when a specific use case has presented itself.

## Development

For a test build using cargo:
```bash
cargo build
```

To build a distributable wheel:
```bash
maturin build --release
```
