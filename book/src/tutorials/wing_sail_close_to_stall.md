# Wing sail close to stall

The Lifting line theory was originally developed to model wings that can be described by potential flow. That is, it is usually assumed that there is a relatively simple linear relationship between lift and angle of attack on each line element. This is a reasonable assumption in many practical situations, but not always. In particular, wing sails tend to operate very close to stall - to maximize the thrust - where the lift is highly affected by viscous effects. 

The goal with Stormbird is to be able to capture the physics of wing sail even in these challenging conditions. However, this is not trivial, and requires some care when executing a simulation. Similar problems are reported for other similar implementations, such as the lifting line method described in Gallay et al (2015). They also describe potential solutions, which has also inspired one of the solutions implemented in Stormbird.

This chapter explains how to deal with close-to-stall cases.

As of writing this, Stormbird is still under development. New features that better capture the close-to-stall physics may very well be added at a later stage. This chapter will be kept up to date with current best practices on this topic. 

## Challenge

As described in [the solved chapter](../lifting_line/solver.md), Stormbird implements a non-linear solver to find the circulation distribution, which, in principle, should be able to handle non-linear relationships between lift and angle of attack. However, even though the non-linear solver is more stable than a linearized version, it can still lead to unstable conditions.

## References
