# Simulation methods

This chapter gives references to papers and other literature that use similar or identical methods as the ones implemented in Stormbird. These can be read to gain better understanding of the underlying methods, or to find alternative implementations to the Stormbird library.

## Publications using Stormbird

Articles that use the Stormbird library directly is shown in the sub sections below.
#### Sail-induced resistance on a wind-powered cargo ship, 2022
By J. V. Kramer and S. Steen. A larger article that was really about the hydrodynamic modeling of wind-powered ships. However, an early version of the lifting line method implemented in Stormbird was used for the sail modeling. It also contains a simple validation experiment that compares lifting line simulations against CFD. Can be downloaded [here](https://doi.org/10.1016/j.oceaneng.2022.111688).

#### Actuator line for wind propulsion modelling, 2023
By J. V. Kramer and S. Steen. Compares CFD, lifting line and actuator line simulations against each other for a case that includes two wing sails in close proximity. The actuator line simulations also contains the effect of a superstructure.Can be downloaded [here](https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling).

#### Optimizing Wing Sails with Actuator Line Simulations and an Effective Angle of Attack Controller, 2025
By J. V. Kramer. Further testing of the actuator line model, as well as a first test of a simple controller for the angle of attack for a wing sail. Can be downloaded [here](https://www.researchgate.net/publication/396775692_Optimizing_Wing_Sails_with_Actuator_Line_Simulations_and_an_Effective_Angle_of_Attack_Controller).

## Lifting line
### Open source projects
Below is a list of other open source lifting line implementations that have similar functionality as Stormbird

#### CN-AeroModels
An open source implementation of a discrete static lifting line, intended for wind propulsion modeling. The capabilities of this software are very similar to the static lifting line implementation in Stormbird. Can be found [here](https://gitlab.com/lheea/CN-AeroModels).

#### MachUpX
An open source implementation of a discrete static lifting line, intended for wings with and without sweep. Can be found [here](https://github.com/usuaero/MachUpX).

### Books
Some books that might be useful for those interested in learning more about the fundamentals of the lifting line method.

#### Low-Speed Aerodynamics, 2001
By J. Katz and A. Plotkin. A book that focuses very much on potential theory for air craft applications. Includes a many details about lifting line theory.

#### Fundamentals of Aerodynamics, 2005
By J.D Anderson. A book about many things within aerodynamics, especially for airplanes, that also have an excellent chapter about the lifting line method and how to implement a basic straight forward version of it.



### Papers
Below is a list of papers that either use lifting line methods for sail modeling or serves as direct inspiration for the lifting line method implemented in Stormbird.

#### VSAERO theory document, 1987
By B. Maskew. A detailed description of the theory behind the panel code VSAERO. This includes detailed formulations for how to compute lift-induced velocities for discrete vortex lines. Can be downloaded [here](https://ntrs.nasa.gov/api/citations/19900004884/downloads/19900004884.pdf)

#### Modern Adaptation of Prandtl's Classic Lifting-Line Theory, 2000
By W. F. Phillips et al. First known example of a discrete lifting line, as in, capable of modeling multiple wings in the same simulation. Can be downloaded [here](https://arc.aiaa.org/doi/abs/10.2514/2.2649?journalCode=ja).

#### A Numerical Lifting-Line Method Using Horseshoe Vortex Sheets, 2011
By W. F. Phillips. Another example of a discrete lifting line implementation. Can be downloaded [here](https://digitalcommons.usu.edu/spacegrant/2011/Session1/4/).

#### Numerical analysis of multiple, thin-sail geometries based on Prandtl’s lifting-line theory, 2013
By R. E. Spall et al. A paper about modeling sails using lifting line theory. The output is compared against other potential theory methods with good results. Can be downloaded [here](https://www.sciencedirect.com/science/article/pii/S0045793013001606).

#### Local Results Verification of a 3D Non-Linear Lifting Line Method for Fluid-Structure Interactions Simulation on a Towing Kite for Vessels, 2017
By C. Duport et al. Example of a discrete lifting line being used for towing kite modeling. Can be downloaded [here](https://www.researchgate.net/profile/Kostia-Roncin/publication/330114548_Local_Results_Verification_of_a_3D_Non-Linear_Lifting_Line_Method_for_Fluid-Structure_Interactions_Simulation_on_a_Towing_Kite_for_Vessels/links/5c2e2f41299bf12be3ab2165/Local-Results-Verification-of-a-3D-Non-Linear-Lifting-Line-Method-for-Fluid-Structure-Interactions-Simulation-on-a-Towing-Kite-for-Vessels.pdf).

#### Practical Implementation of a General Numerical Lifting-Line Method, 2021
By C. Goates. Another discrete lifting line method, with particular focus on how to deal with swept wings. Can be downloaded [here](https://www.researchgate.net/publication/348240031_Practical_Implementation_of_a_General_Numerical_Lifting-Line_Method).
#### Rapid aerodynamic method for predicting the performance of interacting wing sails, 2023
By K. Malmek et al. About a lifting line method that employs a practical simplification; rather than solving for the vortex strength on the individual line elements, the lift and drag on a single sail is computed directly from the simplified elliptic wing equations for 3D effects, as a function of the local angle of attack. Interaction effects between multiple sails are still computed using a discrete lifting line approach, but with where the circulation distribution is assumed to be elliptic. This allows for a quicker solution, but still with good results. A similar simulation type is possible in Stormbird, by using a "prescribed circulation" with an elliptical shape. Can be downloaded [here](https://www.sciencedirect.com/science/article/pii/S0029801823029803?via%3Dihub).

## Actuator line

### Papers
#### Numerical Modeling of Wind Turbine Wakes, 2002
By J. N. Sørensen and W. Z. Shen. As far as we know, the first paper about actuator line modeling. Introduces the method in a general sense. Can be downloaded [here](https://asmedigitalcollection.asme.org/fluidsengineering/article/124/2/393/444521/Numerical-Modeling-of-Wind-Turbine-Wakes).

#### Actuator Disc Methods Applied to Wind Turbines, 2004
By R. F. Mikkelsen. A PhD thesis that include investigations of actuator line modeling. The thesis introduces the concept of using a 'planar force distribution' that is constant along a line section. The same principle is used in Stormbird. Can be downloaded [here](https://backend.orbit.dtu.dk/ws/portalfiles/portal/5452244/Robert.PDF).

#### A Comparison of Actuator Disk and Actuator Line Wind Turbine Models and Best Practices for Their Use, 2012
By L A. Martinez et al. A paper that tests different settings for actuator line simulations. Can be downloaded [here](https://www.researchgate.net/publication/271374677_A_Comparison_of_Actuator_Disk_and_Actuator_Line_Wind_Turbine_Models_and_Best_Practices_for_Their_Use).

#### An Advanced Actuator Line Method for Wind Energy Applications and Beyond, 2017
By M. Churchfield et al. Introduces two "advanced" functionalities for actuator line simulations (both of which are also available in Stormbird): the concept of an anisotropic force distribution and weighted integral sampling of the control point velocity. Results from this paper suggest that the shape of the force projection can have a large influence on the accuracy, and that an anisotropic force projection is more correct than an isotropic one. Can be downloaded [here](https://www.nrel.gov/docs/fy17osti/67611.pdf).

#### Spanwise Flow Corrections for Tidal Turbines, 2018
By A. Wimshurst and R. Willden. A paper that discusses different spanwise flow corrections in the context of using actuator line models for tidal turbines. Can be downloaded [here](https://www.researchgate.net/publication/336979652_Spanwise_flow_corrections_for_tidal_turbines).

#### A new tip correction for actuator line computations, 2020
By K. O. Dag and J. N. Sørensen. A paper that introduces a new tip correction method for actuator line simulations based on lifting line theory. They suggest to run to lifting line calculations in parallel with the actuator line simulation; one with a viscous core length equal to the force projection width, and one with a very small core length. The difference in velocity can then be used to correct the raw sampled velocity from the CFD grid. The same method is available in Stormbird. Can be downloaded [here](https://onlinelibrary.wiley.com/doi/epdf/10.1002/we.2419).

#### A RANS-BEM Method to Efficiently Include Appendage Effects in RANS-Based Hull Shape Evaluation, 2021
By H. Renzsch et al. A paper about the use of actuator lines to model a sail boat keel.Can be downloaded [here](https://www.researchgate.net/publication/350027407_A_RANS-BEM_Method_to_Efficiently_Include_Appendage_Effects_in_RANS-Based_Hull_Shape_Evaluation).
