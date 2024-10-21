# Tutorials

This chapter gives an introduction to the tutorials made for the library. At the moment, only the lifting line functionality is covered for a limited set of use cases. However, more will come later. 

This text is only meant as a high-level overview. To get the full picture, it is necessary to look at the detailed code and input files for each tutorial. These files should be distributed along with this book. At the time of writing this (October 2024), the code repository is not yet open. However, this book, along with code documentation and tutorial files is distributed using a Teams-site for project partner in the research project KSP WIND. On that site, there should be a folder named `examples` with sub-folders for each example.

## FMU vs Python
The current tutorials focuses on how to run simulations using the Python interface. However, running FMU simulations are similar. The main difference is connected to how FMUs work relative to Python code. This will be expanded upon in the future.

## Overview of Python tutorial tutorials
Below is a list of the available tutorials, with a short description of what they do. The text in **bold** refers to sub-folders within the main example folder.

- **1 - Section models:** This folder contains examples of how to generate section models for Stormbird. It covers both manual setup and how to tune a model based on some external data source, such as CFD simulations.
- **2 - Single wing sail:** This folder contains examples of how to simulate wing sails, both with a single element and multiple element. It also shows different ways of executing the same simulation, and compares the output from different simulation modes against each other. For the case of a single element foil, there is also a comparison against experimental data.
- **3 - Single rotor sail:** This folder contains an example demonstrating how to simulate rotor sails. Rotor sails are a challenging sail type, and require some empirical corrections to work accurately. The example uses multiple correction methods, and compare the output from each method.
- **4 - Moving wing:** This example covers how to simulate a heaving wing, both with a dynamic simulation and with quasi-static simulation. The results are compared against simplified theory of heaving wings.
- **5 - wing sail interaction:** This example reproduces the lifting line results from the interaction effects study found int he paper ["Actuator line for wind 
propulsion modelling"](https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling).