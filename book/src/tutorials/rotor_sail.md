# Rotor sail

## Challenges
Rotor sails are special in several ways - relative to other lifting surfaces - which pose challenges from a simplified modelling perspective. 

The first challenge is that they tend to operate with extremely high lift coefficients. As a consequence, the lift-induced velocities in the proximity of a rotor sail is much stronger than for a conventional wing, such as a wing sail. This can sometimes cause numerical instabilities, for instance when integrating the shape of a dynamic wake. In addition, the shape of the wake tend to affect the lift-induced velocities more than for a conventional wing, which has consequences for quasi-static modelling.

The second challenge is that both the lift and drag is usually affected by strong viscous effects. A stationary circular cylinder will have massive amounts of flow separation in its wake, which causes high drag forces. The amount of flow separation will decrease if the cylinder starts to rotate, but even at practical spin ratios, there can be strong viscous effects on the wake. This is different from wing sails, where viscous effects only play a major role when the sails are operated close to or above stall. This can both create instabilities and require more empirical modelling than a conventional wing.

Third, a rotor sail will often use geometrical structures intended to disrupt the lift-induced flow from the rotors, which are not directly captured by a lifting line model. This can for instance be end disks or specially designed foundations. It is important to accurately capture the effect of these structures, and modelling them will require some amount of empirical correction.

Last, but not least, the lift on a rotor sail section have a fundamentally different dependency on the lift-induced velocities than a foil section or a suction sail section. In both of the latter cases, the amount of lift on a section will depend on the lift-induced angle of attack. In general, the larger the lift-induced velocities, the smaller the lift from the section. This naturally reduces the circulation distribution on ends of the wings, which results in numerically well behaved smooth circulation distributions. For a rotor sail, on the other hand, the lift is only dependent on the rotational speed and incoming velocity, not the angle of attack. If the lift-induced velocities become large, the lift on a single section will rotate, but the magnitude will not necessarily decrease. As a consequence, there are no mechanisms to cause smooth circulation distributions on a rotor sail in a lifting line simulation. This is problematic for two reasons: 1) it seems to differ from the circulation distribution predicted with CFD simulations, which indicates that some of the physics is lacking and 2) it may cause numerical issues

## What is the point then?
In later sections, there will be given instructions on how deal with the challenges described above. This can be summarized as applying some amount of *stabilizing tricks* in the simulation setup, and use empirical corrections to capture physical effects not directly modelled by the lifting line model. When reading about these tricks and corrections, you might wonder, what is the point of running lifting line simulations of a rotor sail, if so many corrections are needed? Would it not be easier to just use a pure empirical model for early stage analysis, and then do CFD simulations at later stages?

It is true that a lifting line model **alone** is **less physical accurate for rotor sails than for other sail types**. The best practice for a rotor sail simulations is essentially to use a combination of lifting line simulations and external data from either experiments or CFD simulations. That is, if the goal is to model a single rotor, standing on a simple flat surface, a lifting line simulation might not be the right tool. Since the model must be tuned using high-fidelity data it might be easier to just use the tuning data directly, for instance with a simple interpolation method or regression model. However, a single rotor on a flat surface is a **simplified case**.  

A lifting line simulation of a rotor sail is a potential candidate for capturing the physics in more advanced situations. In many ways, the method can be a practical way to use and correct the high-fidelity data in **situations not directly tested in CFD or experiments**. This can be exemplified in three ways:

### Point 1 - sail-to-sail interaction effects
As will be highlighted further under best practices, a lifting line simulation of a rotor sail is created by tuning some model parameters to high fidelity data of a single sail. When tuning the model, two things happen.

### Point 2 - spatially varying velocity fields

### Point 3 - motions

### When not to use lifting line simulations for rotor sails

## Best practices

## Model setup

## Empirical data sources