# Simulation methods

This chapter gives references to papers and other literature that use similar or identical methods as the ones implemented in Stormbird. These can be read to gain better understanding of the underlying methods, or to find alternative implementations to the Stormbird library.

## Publications using Stormbird

Articles that use the Stormbird library directly is shown in the sub sections below. 
- **[Sail-induced resistance on a wind-powered cargo ship](https://www.sciencedirect.com/science/article/pii/S0029801822010447)** (2022), by J. V. Kramer and S. Steen. A larger article that was really about the hydrodynamic modelling of wind-powered ships. However, an early version of the lifting line method implemented in Stormbird was used for the sail modelling. It also contains a simple validation experiment that compares lifting line simulations against CFD.
- **[Actuator line for wind propulsion modelling](https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling)** (2023), by J. V. Kramer and S. Steen. Compares CFD, lifting line and actuator line simulations against each other for a case that includes two wing sails in close proximity. The actuator line simulations also contains the effect of a superstructure.

## Lifting line
### Open source projects
- **[CN-AeroModels](https://gitlab.com/lheea/CN-AeroModels)**. An open source implementation of a discrete static lifting line, intended for wind propulsion modelling. The capabilities of this software are very similar to the static lifting line implementation in Stormbird.

### Papers
- **[Modern Adaptation of Prandtl's Classic Lifting-Line Theory](https://arc.aiaa.org/doi/abs/10.2514/2.2649?journalCode=ja)** (2000) by W. F. Phillips and more. First known example of a discrete lifting line, as in, capable of modelling multiple wings in the same simulation. 
- **[Rapid aerodynamic method for predicting the performance of interacting wing sails](https://www.sciencedirect.com/science/article/pii/S0029801823029803?via%3Dihub)** (2023) by K. Malmek and more. About a lifting line method that employs a practical simplification; rather than solving for the vortex strength on the individual line elements, the lift and drag on a single sail is computed directly from the simplified elliptic wing equations for 3D effects, as a function of the local angle of attack. Interaction effects between multiple sails are still computed using a discrete lifting line approach, but with where the circulation distribution is assumed to be elliptic. This allows for a quicker solution, but still with good results. A similar simulation type is possible in Stormbird, by using a "prescribed circulation" with an elliptical shape. 
- **More to come!**

## Actuator line

### Papers
**To come!**