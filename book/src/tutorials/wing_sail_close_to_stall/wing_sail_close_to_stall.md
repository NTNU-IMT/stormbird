# Wing sail close to stall

The Lifting line theory was originally developed to model wings that can be described by potential flow. That is, it is usually assumed that there is a relatively simple linear relationship between lift and angle of attack on each line element. This is a reasonable assumption in many practical situations, but not always. In particular, wing sails tend to operate very close to stall - to maximize the thrust - where the lift is highly affected by viscous effects. 

The goal with Stormbird is to be able to capture the physics of wing sail even in these challenging conditions. However, this is not trivial, and requires some care when executing a simulation. Similar problems are reported for other similar implementations, such as the lifting line method described in Gallay et al (2015). They also describe potential solutions, which has also inspired one of the solutions implemented in Stormbird.

This chapter explains how to deal with close-to-stall cases.

As of writing this, Stormbird is still under development. New features that better capture the close-to-stall physics may very well be added at a later stage. This chapter will be kept up to date with current best practices on this topic. 

## Challenge

As described in [the solved chapter](../lifting_line/solver.md), Stormbird implements a non-linear solver to find the circulation distribution, which, in principle, should be able to handle non-linear relationships between lift and angle of attack. However, even though the non-linear solver is more stable than a linearized version, it can still lead to unstable conditions. As such, if the it is interesting to simulate forces on the wing sails when the sails are operated very close to stall, it is likely necessary with some amount of *artificial stabilization* techniques.

## Comparison data

The numerical examples presented in this chapter will be compared against results from Graf et al. (2014). The paper contains both experimental and numerical results, where the numerical results are both from CFD and a similar lifting line method as the one used in Stormbird. 


## References
- Graf, K., Hoeve, A.V., Watin, S., 2014: Comparison of full 3D-RANS simulations with 2D-RANS/lifting line method calculations for the flow analysis of rigid wings for high performance multihulls. Ocean Engineering, Volume 90. [Link](https://doi.org/10.1016/j.oceaneng.2014.06.044)
