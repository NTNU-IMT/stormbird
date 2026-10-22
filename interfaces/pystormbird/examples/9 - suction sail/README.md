# Suction sail based on Turbosail

This example shows how to tune a model of a suction sail to correspond to the original Turbosail, using the `ActiveAirFlowFoil` section model with the `PowerToInternalFlow` conversion. The data is digitized from Charrier et al. (1985), *Fondation Cousteau and windship propulsion 1980 - 1985, System Cousteau - Pechiney*, Journal of Wind Engineering and Industrial Aerodynamics, 20, 39-60.

## Tuning force model
The lift and drag model has so far been tuned manually (could probably be automatized later), assuming fixed suction coefficients for the entire sail, that match the experimental data. Only the 3D data is used for tuning. 

## Tuning the power model

The power curve gives the lift coefficient as a function of the power coefficient, but not the angle of attack or the suction coefficient for each point. To tune the power model, it is assumed that the sail is operated at the maximum lift for each suction coefficient, which is the lowest suction coefficient that can give the lift.

An alternative is to assume that the combination of angle of attack and suction coefficient that minimizes the drag was used. However, more suction always gives lower drag for the same lift in the data, so this assumption gives the largest suction coefficient for most points, independent of the power, and can not be used to tune the power model.

The tuning is done in `tune_power_model.py`. The porous fraction is set to 0.25 and the fan efficiency to 0.7, while the loss coefficient and the external pressure at zero lift (`cp_zero_cl`) are tuned. Only the ratio between the porous fraction and the square root of the loss coefficient affects the flow, so the porous fraction is set from the geometry, while the loss coefficient is tuned. The effect of the lift on the external pressure (`cp_gradient`) is fixed at zero: along the curve of maximum lift, the lift coefficient increases almost linearly with the suction coefficient, so `cp_zero_cl` and `cp_gradient` have close to the same effect on the power, and can not be found separately from this data.

With a given suction coefficient on each line element, the power is linear in the loss coefficient and `cp_zero_cl`, so they are found with linear least squares, using simulations with a constant suction coefficient along the span, at the angle of attack with maximum lift. The relative error in the power, compared to the experimental power curve at the same lift, is minimized. The tuned model is then verified with simulations where the power is the internal state, and where the suction coefficient on each line element is found from the internal flow model.

The lift and drag model only has data for suction coefficients between 0.027 and 0.05. Outside this range, the foil for the closest suction coefficient is used, so the model can not represent the lower part of the power curve, where the lift is below the maximum lift for a suction coefficient of 0.027.
