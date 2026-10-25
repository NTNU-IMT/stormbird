# Pystormbird examples
The examples in this folder shows various ways of using both the [pystormbird](../../pystormbird/) library and the [stormbird_setup](../../stormbird_setup/) library. Therefore, both of these libraries must be installed to make them work.

## Setting up the environment
The dependencies for the examples are defined in `pyproject.toml` in this folder, and the easiest way to install them is with [uv](https://docs.astral.sh/uv/). Run the following in this folder:

```
uv sync
```

This creates a virtual environment in the `.venv` folder, with all the packages that are needed. Both `pystormbird` and `stormbird_setup` are installed from the source code in this repository. As `pystormbird` is compiled from Rust, a Rust compiler must be installed, and the first installation takes a few minutes.

The examples can then be executed with `uv run` from the folder of each example, for instance:

```
cd "3 - rotor_sails"
uv run single_rotor_multiple_spin_ratios.py
```

Alternatively, activate the environment with `source .venv/bin/activate` and use `python` directly.

Changes to `stormbird_setup` are used directly, as it is installed in editable mode. Changes to the Rust code in `pystormbird` or the core library require a rebuild:

```
uv sync --reinstall-package pystormbird
```

## Plots
All plots are made with [matplotlib](https://matplotlib.org/), and data files are read with [polars](https://pola.rs/). Matplotlib needs a GUI backend to show the plots in a window, which is often missing, for instance under WSL. The plots are then not shown, without any error. A Qt backend can be included in the environment with:

```
uv sync --group gui
```
