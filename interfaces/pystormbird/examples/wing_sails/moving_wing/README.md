# Moving wing

This example demonstrates how to simulate a wing that moves during a simulation, and the difference between a quasi-steady and a dynamic wake when the motion is unsteady.

The wing is a long, rectangular wing with an aspect ratio of 32, which moves harmonically up and down (heave) normal to the freestream. The motion gives a time-varying effective angle of attack, and therefore a time-varying lift. The simulation is run for two reduced frequencies, $k = \omega b / U$, where $\omega$ is the radial frequency of the motion, $b$ is half the chord length and $U$ is the freestream velocity.

## What the example demonstrates

- **How to move a wing in a simulation.** The position of the wing is updated before each time step with `set_translation_with_velocity_using_finite_difference`, which also computes the velocity of the wing. The velocity of the wing is felt as a change in the inflow, which is how the motion affects the lift.
- **The effect of the dynamic wake.** In a quasi-steady simulation, the lift follows the effective angle of attack instantly. In a dynamic simulation, the vorticity shed into the wake when the circulation changes reduces the amplitude of the lift and delays it relative to the motion. The effect is larger for higher reduced frequencies.
- **A comparison against theory.** The dynamic simulations are compared against Theodorsen's model for the circulatory lift on a foil in harmonic motion, which includes both the reduction in the lift amplitude and the phase shift.

The resulting plot shows the lift coefficient as a function of time for the dynamic simulations (solid lines), the quasi-steady simulations (dashed lines) and Theodorsen's model (dotted lines), with one color for each reduced frequency.

### About the comparison with Theodorsen's model

Theodorsen's model is a simplified, two-dimensional model, so the match with the simulations is not expected to be exact. Some differences to be aware of:

- The model is two-dimensional, while the simulated wing is three-dimensional. To account for this, the quasi-steady lift slope in the model is taken from the quasi-steady simulation, rather than using the two-dimensional value of $2 \pi$. The comparison therefore shows the effect of the unsteady wake only.
- The model assumes a flat wake that starts at the trailing edge of the foil and extends infinitely far downstream. In the lifting line simulation, the wake starts close to the line that represents the wing. The phase shift in the simulation is somewhat larger than in the model, and depends on the length of the first wake panel.
- The model gives the periodic solution after any start-up effects have died out. The simulations start from rest, so the first part of the time series includes a start-up transient.
- Only the circulatory lift is included in the model. Added mass effects are not included in either the model or the simulation.

## Files

- `heaving_wing.py`: The main script. It runs the quasi-steady and dynamic simulations of the heaving wing for each reduced frequency, computes the lift from Theodorsen's model, and plots the results. Options:
    - `--visualize-wake`: Shows a live animation of the wake for each simulation while it runs. Requires [pyvista](https://pyvista.org/), which can be installed with `pip install pyvista`.
    - `--write-plots`: Writes the plot to `heaving_wing.pdf`, `heaving_wing.svg` and `heaving_wing.png`.
- `theodorsen.py`: Implementation of Theodorsen's model, using analytical expressions. `theodorsen_function` computes Theodorsen's function, $C(k) = F(k) + i G(k)$, from Hankel functions, and `heave_circulatory_lift_coefficient` computes the circulatory lift coefficient for a foil in harmonic heave motion, where the amplitude is reduced by $|C(k)|$ and the phase is shifted by $\arg C(k)$ compared to the quasi-steady lift.
- `wake_visualization.py`: Live visualization of the wake with pyvista, used by the `--visualize-wake` option. The wake is read from the simulation with the `wake_as_vtk` function after each time step, and colored by the strength of the vortex elements.
