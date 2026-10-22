# Simulation of rotor sails

This example demonstrates how rotor sails can be simulated, including how to use different correction techniques.

Each example is compared against data from experiments that tested rotor sails in a wind tunnel.

## Scripts

- `setup.py`: Shared setup of the simulations, used by all the other scripts. It builds a lifting line model of one or more rotor sails, runs either a quasi-steady or a dynamic simulation, and returns the force coefficients together with the circulation and angle of attack along the span of each rotor.
- `single_rotor_multiple_spin_ratios.py`: Runs a single rotor sail for a range of spin ratios, with different solver settings, and compares the lift and drag against the experiments by Deybach et al. (2024).
- `single_rotor_single_case.py`: Runs a single rotor sail at one spin ratio, given with the `--spin-ratio` argument, and plots the circulation and the effective angle of attack along the span for the different solver settings.
- `two_rotors_interference_factors.py`: Computes the interference factors for two rotor sails, where one rotor is placed at different angular positions around the other, and compares them against the experiments by Deybach et al. (2026). The interference factors are the lift and drag on the first rotor divided by the lift and drag on the same rotor when it stands alone. See the section below for more.

## The viscous core length evolution and rotor-to-rotor interaction

The point of `two_rotors_interference_factors.py` is to show the importance of the `viscous_core_length_evolution` in the dynamic wake model.

Real vortices have a viscous core, which grows when the vortex moves away from the rotor. In a lifting line model, the wake is represented by vortex lines, and the viscous core is modeled with a viscous core length, which limits the induced velocities close to the vortex lines. With the `SinIncrease` evolution, the viscous core length increases along the wake, from the value at the rotor to a final value. The `evolution_length_factor` sets how far into the wake the final value is reached, as a fraction of the wake length. A lower value therefore means that the core grows faster.

This effect is found to be very important when one rotor or sail stands directly in the wake of another, which happens when the interfering rotor is upstream, at angular positions close to 0 degrees. Without a growing viscous core, the downstream rotor experiences the very concentrated tip vortices from the upstream rotor, which is not realistic.

The script compares three variants, with different line styles in the plot:

1. A quasi-steady simulation, which has no viscous core length evolution.
2. A dynamic simulation with an evolution length factor of 1.0, i.e., a slow growth of the viscous core.
3. A dynamic simulation with an evolution length factor of 0.3, i.e., a fast growth of the viscous core.

The dynamic simulations use a steady state strength update and no dynamic wake shape. That is, the shape of the wake is not affected by the induced velocities, as in the quasi-steady simulation, so that the difference between the variants is mainly the viscous core length evolution. Note that the dynamic simulations take considerably longer to run than the quasi-steady simulation.
