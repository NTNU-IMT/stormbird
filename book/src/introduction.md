# Introduction

Welcome to the Stormbird book!

Stormbird is a library for simulating lifting surfaces, i.e. wings, under the assumption that they can be represented as *line-models*. Although this makes it usable for a variety of different cases, it is also mostly developed to offer efficient modeling of modern wind propulsion devices. That is, the following types of lifting surfaces are of particular interest:

1) Wing sails
2) Rotor sails
3) Suction sails
4) Kites

These use cases require modeling of strong viscous effects on the lift (e.g., due to large angles of attack), various lift generation mechanisms (classical foils, with and without flaps, rotating cylinders, foils with boundary layer suction), interaction between multiple lifting surfaces (thereare often multiple sails), interaction between lifting surfaces and other structures (the superstructure and other deck structures) and unsteady effects (kites flying dynamically, analysis of ships operating in waves, maneuvering simulations).

At the same time, it is also often necessary with efficient computations. The user will usually be interested in testing many different weather conditions, ship speeds, sail configurations, and operational variables. The goal is, therefore, to find the right balance between accuracy and speed for the intended use case. To achieve this, the library supports the following methods, that offer different levels of complexity and computational speed:

 1) Discrete static lifting line, for steady- or quasi-steady cases
 2) Discrete dynamic lifting line, for unsteady or steady cases with large wake deformations
 3) Actuator line, for steady and unsteady cases where interaction with other structures is of interest

The library is developed as part of the research project KSP WIND by the Department of Marine Technology at the [Norwegian University of Science and Technology](https://www.ntnu.edu/). The main developer is Jarle Vinje Kramer.

## Who the Book is for
You should read this if you are interested in using Stormbird to run lifting line or actuator line simulations, or if you just want more information on the theory behind each method. The book is written primarily for *users*, and are therefore not focused on the underlying source code. However, developers should off course read this as well, to understand the intended use case.

## How to Use This Book
The books is organized in two main parts.

The first part gives an introduction to the theory and the models available as well as overall concepts in the implementation. This is intended to give a birds eye view of the library and the functionality. There will also be references to other literature for more details when that is appropriate. 

The second part gives more detailed examples and tutorials on how the library can be used in a practical sense. This sections will focus on concrete use cases and be as specific as possible.

## Overview of Different Flavors
Stormbird itself is a [Rust](https://www.rust-lang.org/) library. Rust is a nice programming language that offers a unique combination of high computational speed, and a modern user friendly developer experience. However, there are also two other ways to use the functionality without knowing how to program in Rust, listed below:
- Through a `Python` interface, which is facilitated by `pystormbird`. 
- Run `FMI/FMU` simulation based on the `StormbirdLiftingLine` `FMU`.

The book will cover both approaches as much as possible, as most of the functionality is shared between them.