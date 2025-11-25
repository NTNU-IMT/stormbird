# Test of dynamic stall hysteresis
This example aims to reproduce the results presented in the paper [Wind Tunnel Tests of a Two-Element Wingsail with Focus on Near-Stall Aerodynamics](https://onepetro.org/JST/article/9/01/110/569569/Wind-Tunnel-Tests-of-a-Two-Element-Wingsail-with). It demonstrates how the exact stall angle for a wing sail is dependent on hysteresis effects, meaning that the stall angle is not a static quantity, but rather a value that depends on the history of angles of attack.

The exact source of hysteresis effects on the stall characteristics is not entirely certain, but it seems likely that at least part of it is that there could be several possible circulation distributions for a given angle of attack. Which one that ends up as the current one depends on how the sail is controlled to the target angle of attack. In other words, it matters wether you start from a small angle of attack and then increases it, or from a large angle of attack and then decreases it. 

The tests in this folder sees how this behavior is captured in Stormbird.
