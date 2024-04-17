# Introduction

Welcome to the Stormbird book. Great that you are interested in learning more about this software project. 

Stormbird is a library for simulating lifting surfaces, i.e. wings, under the assumption that they 
can be represented as *line-models*. Although this makes it usable for a variety of different cases, 
it is also mostly developed to offer efficient modeling of modern wind propulsion devices. That is, 
the following types of lifting surfaces are of particular interest:

1) Wing sails
2) Rotor sails
3) Suction sails
4) Kites

These use cases require modeling of strong viscous effects on the lift (e.g., due to large angles of 
attack), various lift generation mechanisms (classical foils, with and without flaps, rotating 
cylinders, foils with boundary layer suction), interaction between multiple lifting surfaces 
(when there are multiple sails), interaction between lifting surfaces and other structures (the 
superstructure and other deck structures) and unsteady effects (kites flying dynamically, analysis 
of ships operating in waves, maneuvering simulations).

At the same time, it is also often necessary with efficient computations. The user will usually be 
interested in testing many different weather conditions, ship speeds, sail configurations, and 
operational variables. The goal is, therefore, to find the right balance between accuracy and speed 
for the intended use case. To achieve this, the library supports the following methods, that offer 
different levels of complexity and computational speed:

 1) Discrete static lifting line, for steady- or quasi-steady cases
 2) Discrete dynamic lifting line, for unsteady or steady cases with large wake deformations
 3) Actuator line, for steady and unsteady cases where interaction with other structures is of 
 interest


## Who the Book is for
You should read this if you are interested in using Stormbird to run lifting line or actuator line
simulations, or if you just want more information on the theory behind each method. The book is 
written primarily for *users*, and are therefore not focused on the underlying source code, but 
rather how to use the models (however, developers should off course read this as well, to understand 
the intended use case...)

## How to Use This Book
The books is organized in three main parts.

The first part gives an introduction to the theory and the models available. There will be 
references to other literature when that is appropriate. 

The second part gives examples and tutorials on how the library can be used in a practical sense. 

The third and last part gives an overview of the validation tests that have, so far, been done for 
the methods. If you want to use Stormbird for something that is not directly covered in the 
validation tests, it is recommended that you do your own testing first. This section is also 
intended to be extended when new validation cases are made.

## Overview of Different Flavors
Stormbird itself is a Rust library. However, there are multiple ways to use this library:
- Through a `Python` interface, which is facilitated by `pystormbird`. 
- Run `FMI/FMU` simulation.

