# Interfaces to the Stormbird library

The following interfaces currently exists:

- [pystormbird](./pystormbird/): a Python library for running the Stormbird library directly. 
- [stormbird_setup](./stormbird_setup/): a Python library for generating JSON strings that can be used as input the Stormbird library. It is not directly connected to the Stormbird core in any way, but exposes different settings as classes that inherits from the Pydantic BaseModel. The main use case is building model files or strings that are read by either the core library or any of the other interfaces.
- [fmus](./fmus/): Functional Mockup Units where the primary one is an FMU of the lifting line functionality
- [cfd_interfaces](./cfd_interfaces/): Makes it possible to use the actuator line functionality in Stormbird in specific CFD codes. At the moment, the only supported CFD code is OpenFOAM, but the interface is also implemented in a way that should allow for codes to use more or less the same principle.