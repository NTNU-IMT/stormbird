# stormbird_setup
A Python library that simplifies the setup of models that are to be used with the stormbird library.

The stormbird library is a Rust crate with a Python interface as well as several other interfaces. More info about this library can be found at the [GitHub page](https://github.com/NTNU-IMT/stormbird)

## Philosophy
The point of this library is to implement the different input structures as Python classes that inherits from the Pydantic base model class. This allows for typed-checked creation of setup data, which is also easily converted to and from json strings, which is often the direct input to Stormbird. 

The Python interface to Stormbird is called pystormbird. However, stormbird_setup does **NOT** require pystormbird to be installed **AND** it is supposed to be agnostic in terms of the flavor of Stormbird that is used. For instance, this library should be equally useful for setting up models for the FMU-version, the Python version, and the OpenFOAM version (actuator line simulations) of Stormbird.

In addition, since the point is to simplify the setup, it also contains simplified builders for *typical* simulations that may be performed with Stormbird.

## Install instructions
A normal Python package. Can be installed by navigating into the folder and execute 

```
pip install .
```

Or, it should also be available on PyPi, and therefore through

```
pip install stormbird_setup
```
