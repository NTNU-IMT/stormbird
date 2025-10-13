# Stormbird
Stormbird is a library for simulating lifting surfaces, i.e. wings, under the assumption that they 
can be represented as *line-models*. Although this makes it usable for a variety of different cases, 
it was primarily developed to offer efficient modeling of modern wind propulsion devices. 

The library is developed by the [Department of Marine Technology](https://www.ntnu.edu/imt) at the [Norwegian University of Science and Technology](https://www.ntnu.edu/) as part of the research project [kSP WIND](https://www.sintef.no/en/projects/2023/wind-enabling-zero-emission-shipping-with-wind-assisted-propulsion/), funded by the [Norwegian Research Council](https://www.forskningsradet.no/en/). The primary developer is [Jarle Vinje Kramer](https://github.com/jarlekramer/).

## Folder structure
The content of the folders in this repository can be described as follows:
- The [stormbird](/stormbird/) folder contains the core library written in Rust
- The [stormath](/stormath/) folder contains implementation of various mathematical utility functions needed for the core library, but which are implemented in a general way so that they may also be useful for other purposes. This includes stuff like spatial vector and matrix data structures, interpolation functionality, solvers etc.
- The [book](/book/) folder contains the source for an mdbook that describes the library and theory more in detail
- The [interfaces](/interfaces/) folder contains various interfaces to use the core library in other languages than Rust. They are generally *high-level*, in the sense that none of them contain access to the entirety of the core library, but some selected functionalities depending on the type of interface. Examples are two Python libraries (one to assist with the setup of models and one for actually running the core library), functional mockup units, and an interface to use the core library within OpenFOAM. 

## Documentation
- A more comprehensive documentation of the library can be found here [LINK TO COME].
- Automatic code documentation for core library here [LINK TO COME]
- Automatic code documentation for the math utilities here [LINK TO COME]

## License
The software is licensed under the GPL3. See [LICENSE](LICENSE) for more.
