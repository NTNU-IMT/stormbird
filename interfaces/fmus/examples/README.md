# Notes on running the examples

The examples are simulated with the [OSP command line interface](https://open-simulation-platform.github.io/cosim). However, the setup and execution of the command line interface is done through a Python script. This is found to be practical, as it simplifies when there are any changes to the setup, and it makes it possible to automatically clean up old simulation files when re-executing an example. 

To make this work, the `cosim` executable must be present on the system path. That is, you should be able to run `cosim` directly in the terminal, without having to specify the entire path to the executable. This can be achieved by adding the `bin` folder in the cosim executable folder to the system path.