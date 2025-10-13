# pystormbird

A Python interface to the Stormbird library.

The intended use cases are as follows:
- Running lifting line simulation through Python scripting
- Testing parts of the code that require plotting and visual inspection

## Implementation details
The interface is made using pyo3: <https://pyo3.rs/>

The layout and structure of the code follow the source code in Stormbird as much as possible. 

The initialization of some classes, such as the DynamicSimulation class, is done using JSON strings, 
to avoid unnecessary maintenance of code. One way to deal with this in a Python script is to use the
JSON functionality in the standard library for dictionaries. 

It is not a goal in itself to offer an interface to every aspect of the Stormbird library. Rather, 
an interface is only made when a specific use case has presented itself (see intended use cases 
above). 

## Install instructions
Can be installed by navigating into the pystormbird folder and execute:
```
pip install . 
```

For a test build, cargo can be used as normal:
```
cargo build
```

To generate a package to distribute:
```
maturin build --release
```