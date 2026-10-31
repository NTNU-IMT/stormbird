# Test of dynamic stall hysteresis
The examples in this folder look at hysteresis effects in the stall, meaning that the lift at a given angle of attack is not a static quantity, but depends on the history of angles of attack. There are two examples:

## Comparison with wind tunnel tests: `two_element_wingsail_wind_tunnel.py`
This example aims to reproduce the results presented in the paper [Wind Tunnel Tests of a Two-Element Wingsail with Focus on Near-Stall Aerodynamics](https://onepetro.org/JST/article/9/01/110/569569/Wind-Tunnel-Tests-of-a-Two-Element-Wingsail-with). It demonstrates how the exact stall angle for a wing sail is dependent on hysteresis effects, meaning that the stall angle is not a static quantity, but rather a value that depends on the history of angles of attack.

The exact source of hysteresis effects on the stall characteristics is not entirely certain, but it seems likely that at least part of it is that there could be several possible circulation distributions for a given angle of attack. Which one that ends up as the current one depends on how the sail is controlled to the target angle of attack. In other words, it matters wether you start from a small angle of attack and then increases it, or from a large angle of attack and then decreases it. 

The angle of attack is changed very slowly in the experiments, so the time lags in the [dynamic stall model](https://jarlekramer.github.io/stormbird_book_pages/sectional_models/dynamic_stall.html) have a very small effect here. By default, the simulation uses dynamic settings with the dynamic stall model. This can be changed with command line arguments:

- `--quasi-steady`: use quasi-steady settings instead. This also turns off the dynamic stall model, as it requires a dynamic simulation.
- `--no-dynamic-stall`: keep the dynamic settings, but turn off the dynamic stall model.

The digitized experimental data is stored in `increasing_cl.csv` and `decreasing_cl.csv`.

## Pitch oscillation through stall: `pitch_oscillation.py`
This example demonstrates the classical hysteresis loop from dynamic stall. The angle of attack on a wing with the default foil model is varied sinusoidally through the stall angle, with a mean value of 18 degrees and an amplitude of 8 degrees. The oscillation is run for several reduced frequencies, `k = ω c / (2 V)`, without the dynamic stall model, with the dynamic stall model but without the leading edge vortex, and with the full dynamic stall model. The results are compared against a quasi-steady sweep over the same range of angles. The results from the last period are plotted, one subplot per frequency.

Without the dynamic stall model, the difference from the quasi-steady results comes from the dynamic wake only. With the dynamic stall model, the stall is delayed when the angle of attack increases and the reattachment is delayed when it decreases, which gives a wider hysteresis loop that grows with the reduced frequency. The leading edge vortex adds extra lift after the lagged angle of attack exceeds the stall angle, which gives the lift overshoot typical of deep dynamic stall, followed by a drop in the lift when the vortex has passed the trailing edge.
