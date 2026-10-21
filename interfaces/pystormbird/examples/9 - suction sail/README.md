# Suction sail based on Turbosail

This example shows how to tune a model of a suction sail to correspond to the original Turbosail, using the `ActiveAirFlowFoil` section model with the `PowerToInternalFlow` conversion. The data is digitized from Charrier et al. (1985), *Fondation Cousteau and windship propulsion 1980 - 1985, System Cousteau - Pechiney*, Journal of Wind Engineering and Industrial Aerodynamics, 20, 39-60.

The example consists of three steps, which must be run in order, as each step uses the result of the previous one:

1) `plot_sectional_data.py`: Fits a foil model to the two-dimensional lift and drag data for each suction coefficient, and combines them to a varying foil model, where the internal state is the suction coefficient. The result is written to `fitted_varying_foil.json`.
2) `plot_3d_data.py`: Simulates the three-dimensional model from the paper with the fitted section model, with the suction coefficient set directly as the internal state, and compares the results to the three-dimensional data.
3) `tune_power_model.py`: Tunes the internal flow model, so that the power given as the internal state matches the power curve from the paper. The tuned section model is written to `tuned_active_air_flow_foil.json`.

The files `suction_sail_data.py` and `suction_sail_model.py` contain functionality for loading the data and running the simulations, which is shared by the three steps.

## Assumptions about the data

- **The two-dimensional drag**: The caption of the figure with the two-dimensional data states that the scale of the drag coefficient is double that of the lift coefficient. The digitized drag values are therefore multiplied with 0.5. Without this correction, the two-dimensional drag would be larger than the three-dimensional drag for the same lift, even though the three-dimensional drag also includes induced drag. The comparison with the three-dimensional data in step 2 supports this correction.
- **The three-dimensional model**: The model in the paper had an aspect ratio of 4, with one end against the wall of the wind tunnel, which gives an effective aspect ratio of 8. This is modeled with a symmetry condition at one end of the wing. The power curve is assumed to be for the same model.
- **The power coefficient**: In the paper, the power coefficient is computed from the measured suction coefficient and the pressure in the internal chamber, as the minimum power needed to suck the air in and discharge it at the speed of the wind. It is therefore not a direct measurement of the power of a fan. In this example, a fan efficiency of 0.7 is assumed, which gives similar values for typical internal pressures.

## Tuning the power model

The power curve gives the lift coefficient as a function of the power coefficient, but not the angle of attack or the suction coefficient for each point. In step 3, it is assumed that the lowest suction coefficient that can give the lift was used for each point, which means that the sail is operated close to the maximum lift for the suction coefficient. 

An alternative is to assume that the combination of angle of attack and suction coefficient that minimizes the drag was used. However, more suction always gives lower drag for the same lift in the data, so this assumption gives the largest suction coefficient for most points, independent of the power, and can not be used to tune the power model.

The porous fraction is set to 0.25 and the fan efficiency to 0.7, while the loss coefficient and the two coefficients for the external pressure are tuned. With a given suction coefficient on each line element, the power is linear in these three parameters, so they are found with linear least squares, using simulations with a constant suction coefficient along the span. The tuned model is then verified with simulations where the power is the internal state, and where the suction coefficient on each line element is found from the internal flow model.

## Results and limitations

- The section model gives a good match with the three-dimensional lift for the two lowest suction coefficients before stall, but over-predicts the lift for the largest suction coefficient. The three-dimensional data shows a much smaller increase in lift between the two largest suction coefficients than the two-dimensional data. The model also stalls at lower angles of attack than the data.
- Only points with a power coefficient below 0.25 are used when tuning, which covers the range that the paper states as relevant for naval applications (below 0.2). For larger power coefficients, the lift in the data increases very little with the power, which the section model does not capture. 
- Within this range, the tuned model matches the lift in the power curve well. Above it, the model predicts a slightly higher lift, and suction coefficients beyond the range of the two-dimensional data.
- The data can not separate the effect of the lift coefficient on the external pressure (`cp_gradient`) from the effect of the suction coefficient, as the lift and the suction coefficient increase together along the power curve. The tuned value of `cp_gradient` therefore ends up at zero, and the dependency on the lift is captured by the other parameters instead.
