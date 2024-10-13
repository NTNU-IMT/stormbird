# Simulation methods

This chapter gives references to papers and other literature that use similar or identical methods as the ones implemented in Stormbird. These can be read to gain better understanding of the underlying methods, or to find alternative implementations to the Stormbird library.

## Publications using Stormbird

Articles that use the Stormbird library directly is shown in the sub sections below. 
- **[Sail-induced resistance on a wind-powered cargo ship](https://www.sciencedirect.com/science/article/pii/S0029801822010447)** (2022), by J. V. Kramer and S. Steen. A larger article that was really about the hydrodynamic modelling of wind-powered ships. However, an early version of the lifting line method implemented in Stormbird was used for the sail modelling. It also contains a simple validation experiment that compares lifting line simulations against CFD.
- **[Actuator line for wind propulsion modelling](https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling)** (2023), by J. V. Kramer and S. Steen. Compares CFD, lifting line and actuator line simulations against each other for a case that includes two wing sails in close proximity. The actuator line simulations also contains the effect of a superstructure.

## Lifting line
### Open source projects
- **[CN-AeroModels](https://gitlab.com/lheea/CN-AeroModels)**. An open source implementation of a discrete static lifting line, intended for wind propulsion modelling.

### Papers

## Actuator line