# Introduction

Welcome to the Stormbird book!

Stormbird is a library for simulating lifting surfaces, i.e. wings, under the assumption that they can be represented as *line-models*, which is a simplified modeling approach. Although this can be usable for a variety of different cases, it is also mostly developed to offer efficient modeling of modern wind propulsion devices. That is, the following types of lifting surfaces are of particular interest:

1) Wing sails
2) Rotor sails
3) Suction sails
4) Kites

To achieve practical modeling capabilities for these use cases, the following physical effects are assumed to be particularly important:
- **Various lift generation mechanisms**: modern sails consists *sections* that range from classical foils, with and without flaps, rotating cylinders and foils with boundary layer suction.
- **Strong viscous effects:** For all lift generating mechanisms above, there will be high lift coefficients with strong viscous effects on both the lift and drag forces. For instance, wing sails tend to be operated close to stall and the lift on a rotating cylinder is strongly affected by partial flow separation.
- **Interaction effects between lifting surfaces:** Many wind powered ships have several sails placed in close proximity. Interaction effects between multiple sails can therefore be important. 
- **Interaction effects with other structures:** Independent of the number of sails, there can in some cases be interaction effects between the sails and other structures on deck, for instance the bridge.
- **Unsteady effects:** Ship applications often require modeling of unsteady effects for instance to model seakeeping behavior or maneuvering. The sail forces are assumed to be important for such cases, which also introduces dynamic effects on the sails themselves. In addition, kites are often flown dynamically to increase the power extracted from the wind.

At the same time, it is also often necessary with efficient computations. The user will usually be interested in testing many different weather conditions, ship speeds, sail configurations, and operational variables. 

The goal is, therefore, to find the right balance between accuracy and speed for the intended use case. To achieve this, the library supports the following methods, that offer different levels of complexity and computational speed:

 1) **Discrete static lifting line**, for steady- or quasi-steady cases
 2) **Discrete dynamic lifting line**, for unsteady or steady cases with large wake deformations
 3) **Actuator line**, for steady and unsteady cases where interaction with other structures is of interest

The library is developed as part of the research project KSP WIND by the Department of Marine Technology at the [Norwegian University of Science and Technology](https://www.ntnu.edu/). The main developer is Jarle Vinje Kramer.

## Developing principle

Stormbird is a library that is primarily made for people with some programming experience. In addition, the goal is also to keep things as simple as possible and avoid [feature creep](https://en.wikipedia.org/wiki/Feature_creep). The idea is that a simple library is easier to maintain and less likely to contain bugs. Last but not least, **computational speed is important** and should not be sacrificed. As such, a guiding principle in the development is [data orientation](https://en.wikipedia.org/wiki/Data-oriented_design). That can mean different things, but in this context it means the following: 

- **Simple data structures:** It is better to use many arrays of simple data structures than a few arrays of complex data structures. 
- **Complexity can often be moved:** In some cases, complexity in the internal software design can be avoided by allowing for more complexity in the setup and post-processing of results. A slightly more complex setup phase is therefore accepted in Stormbird if it simplifies the internal structure. 
- **Don't implement hypothetical use cases:** Stormbird only implements features that are necessary for the use case - meaning simulation of wind power devices. There are many ways to extend Stormbird to also handle other use cases better, but this is not prioritized, at least not at the moment. The goal is to attempt to be **done** at some point. Hypothetical future use cases are a future problem we do not consider today

## Who the Book is for
You should read this if you are interested in using Stormbird to run lifting line or actuator line simulations, or if you just want more information on the theory behind each method. The book is written primarily for *users*, and are therefore not focused on the underlying source code. However, developers should off course read this as well, to understand the intended use case.

## How to Use This Book
The books is intended to give an introduction to the theory and the models available as well as overall concepts in the implementation. The goal is  to give a birds eye view of the library and the functionality. There will also be references to other literature for more details when that is appropriate. 

To truly learn how to use it, though, it is recommended to look at actual input files and code examples. This should be distributed along with the book. See the [tutorial chapter](./tutorials.md) for more on this 

## Overview of Different Flavors
Stormbird itself is a [Rust](https://www.rust-lang.org/) library. Rust is a nice programming language that offers a unique combination of high computational speed, and a modern user friendly developer experience. However, there are also two other ways to use the functionality without knowing how to program in Rust, listed below:
- Through a `Python` interface, which is facilitated by `pystormbird`. 
- Run `FMI/FMU` simulation based on the `StormbirdLiftingLine` `FMU`.

The book will cover both approaches as much as possible, as most of the functionality is shared between them.