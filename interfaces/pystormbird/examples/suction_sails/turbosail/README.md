# Suction sail based on Turbosail

This example shows how to tune a model of a suction sail to correspond to the original Turbosail, using the `ActiveAirFlowFoil` section model with the `PowerToInternalFlow` conversion. The data is digitized from Charrier et al. (1985), *Fondation Cousteau and windship propulsion 1980 - 1985, System Cousteau - Pechiney*, Journal of Wind Engineering and Industrial Aerodynamics, 20, 39-60.

The resulting model is the default suction sail model in `stormbird_setup`, `DefaultSail.SuctionSailCharrier1985`, and the scripts in this example use it directly. The foil parameters, the internal flow model and the simulation settings are therefore set in `stormbird_setup/default_sails/suction_sails.py`, and any update to the model should be made there. The scripts only set up the geometry of the wind tunnel model, with an aspect ratio of 4 and one end against the wind tunnel wall.

The scripts are:

- `compare_3d_lift_and_drag.py`: compares the lift and drag to the 3D data, with the suction coefficient as the internal state.
- `tune_power_model.py`: tunes the internal flow model to the power curve, as described below.
- `compare_default_sail_power_curve.py`: compares the power curve of the default sail model, with the fan power as the internal state, to the experiments.
- `plot_sectional_data.py`: plots the 2D data.

## Tuning force model
The lift and drag model has so far been tuned manually (could probably be automatized later), assuming fixed suction coefficients for the entire sail, that match the experimental data. The 3D data is used for the suction coefficients 0.027, 0.036 and 0.05.

The power curve also contains points with lower suction, so two more foils are added, based on the 2D data:

- A foil without suction, where the lift is based on the 2D data for zero suction. The minimum drag is assumed to be high, as for the other suction coefficients.
- A foil at a suction coefficient of 0.018. The 2D data shows that the suction has a small effect on the lift below this value, and a large effect above it. The stall angle is based on the 2D data, while the other parameters are interpolated between the foil without suction and the foil for a suction coefficient of 0.027. The 2D data shows that the stall angle is close to the value without suction at this suction coefficient, and only increases for larger suction coefficients. Without this, the maximum lift increases steadily from zero suction, rather than showing the threshold in the data.

The lift at zero angle of attack for these two foils is reduced compared to the 2D data, from 1.8 to 1.4 without suction, and from 2.18 to 1.5 at the threshold. The lowest point in the power curve has a lift coefficient of 2.5, which is below the maximum lift in the 2D data without suction, and the power curve indicates that the lift increases very little below the threshold. With the reduced values, the maximum lift of the 3D model is about 2.5 for all suction coefficients below the threshold.

## Tuning the power model

The tuning is done in `tune_power_model.py`. The power curve gives the lift coefficient as a function of the power coefficient, but not the angle of attack or the suction coefficient for each point. It is therefore assumed that the sail is operated at the maximum lift for each suction coefficient. As the power in the model only depends on the suction coefficient, and increases with it, this is also the operating point that gives the lowest power for a given lift, which is assumed to be the goal when the data was made.

An alternative is to assume that the combination of angle of attack and suction coefficient that minimizes the drag was used. However, more suction always gives lower drag for the same lift in the data, so this assumption gives the largest suction coefficient for most points, independent of the power, and can not be used to tune the power model.

The porous fraction is set to 0.25 and the fan efficiency to 0.7. Only the ratio between the porous fraction and the square root of the loss coefficient affects the flow, so the porous fraction is set from the geometry, while the loss coefficient is tuned. The fan efficiency scales the whole power, and can not be found from the power curve.

With a given suction coefficient on each line element, the power is linear in the loss coefficient, `cp_no_suction` and `cp_suction_gradient`, so they can be found with linear least squares, using simulations with a constant suction coefficient along the span, at the angle of attack with maximum lift. The least squares problem is bounded, so that the parameters are always valid in the internal flow model: the loss coefficient must be larger than zero, and the gradient can not be positive. The error in the power is converted to the distance to the experimental curve, normal to the curve, in coordinates scaled with the range of the data. On the flat part of the curve at high power, this is close to the error in the lift, and on the steep part at low power, close to the error in the power. Otherwise, the points where a small difference in one of them gives a large difference in the other would dominate the fit.

The three parameters can not all be tuned, as the effect of the gradient on the power is close to a combination of the effects of the other two parameters. When all three are tuned, or when the external pressure without suction, `cp_no_suction`, is fixed at zero, the loss coefficient ends at the lower bound, i.e., the power curve can only be matched without any loss over the porous surface, which is not physical. `cp_no_suction` is therefore fixed at -0.5, while the loss coefficient and the gradient are tuned. This gives about the same error as the other choices where the loss coefficient is not at the bound, and a lower error than fixing the gradient and tuning `cp_no_suction`. The script prints the result for other choices of fixed parameters as well, to show the effect, and which parameters ended at a bound.

The tuned model is then verified with simulations where the power is the internal state, and where the suction coefficient on each line element is found from the internal flow model. The match is good over most of the power curve. For power coefficients between about 0.05 and 0.1, the lift in the model is up to about 0.4 lower than in the data, as the increase in the lift above the threshold is less steep in the model.
