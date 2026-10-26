# Actuator line simulations of rotor sails

> **Work in progress.** The setup of actuator line simulations of rotor sails is still being developed. The settings in this example are the current best guess, and are likely to change.

This folder contains example scripts that show how to perform actuator line simulations of rotor sails with Stormflow. The goals with the examples are the same as for the lifting line examples found in [`pystormbird/examples/rotor_sails/deybach_wind_tunnel`](../../../../pystormbird/examples/rotor_sails/deybach_wind_tunnel).

There are two main purposes with the examples:

1) Explore different ways to model lift and drag on a single rotor. This type of exploration has been found necessary, as rotor sails are particularly *tricky* when it comes to actuator line simulations (more on that later). The examples in this folder explores how to combine *stabilization techniques*, to handle the special case of rotor sails, and *empirical corrections* to account for errors that is introduced with the stabilization techniques.
2) Check that *tricks* mentioned above still makes it possible to compute interaction effects without any further tuning of the model. That is, any empirical correction applied to a model for a single rotor is seen as OK in the context of actuator line simulations, as long as the same type of modeling can be sued also when the number of rotors are increased or the rotor is moved onto a ship. The current example compares the predicted interaction effects between two rotors in close proximity and compares it against both lifting line results and experimental data.

## The challenge with rotor sails

Rotor sails have three properties that make them a challenging, but interesting, case for actuator line simulations.

First, they create very large lift-forces, which results in very strong wake deformation and large lift-induced velocities. This is in some ways also a challenge for lifting line simulation, but something that does not seem to cause large issues as long as the shape of the wake is not affected by induced velocities. However, in an actuator line simulation, the wake is represented implicitly as part of the velocity and pressure fields, which makes it fully dynamic by default.

Second, unlike both wing sails and suction sails, the circulation distribution on a rotor sail is not dependent on the angle of attack. A spinning cylinder will create more or less the same lift force, independent of how much the wake is deformed. As a result, the circulation distribution is almost constant along the span of a rotor sail, which creates vert strong tip vortices by default. This increases numerical issues considerably, compared to a similar amount of lift, but with a smooth circulation distribution.

In connection to both the points above, all rotor sails come with different forms of end plates to reduce the issues created by the strong tip-vortices. However, strip theory modeling of lifting surfaces cannot model this directly. All end-plate effects must be incorporated in the model using empirical corrections (although some of them may be *semi-physical*)

## Model variants

To handle the challenges explained in the previous section, some amount of *tricks* are necessary. In principle, the goal of the tricks can be divided in two connected points.

1) Handle all numerical instabilities that occur when a very large lift force, combined with an almost constant circulation distribution, create exceptionally strong tip vortices.
2) Apply corrections to the lift-induced drag. At least based on the current best knowledge, all known tricks that reduce the stability issues that arise from a strong tip vortex also reduce the lift-induced drag. In other words, some empirical correction is necessary to recover this loss of accuracy on drag forces.

The scripts in this example folder can be set to use one of several variants of the model settings. The best/recommended one (specified later) is set as default. However, the others are also possible to run, to see how the impact is on the predicted forces. The variants are listed below:

1) No corrections. This variant shows the prediction of lift and lift-induced drag from a pure actuator line model. Since the rotor is modeled with a relatively large projection width (necessary to get stability at large lift-coefficients), this under predicts lift-induced drag severely.
2) The normal, and very physically grounded, trick to correct too low lift-induced drag with actuator line simulations is to active the *lifting line correction* in the model. This will normally recover the drag at a given lift-coefficient. However, activate this type of correction for rotor sails have proven to be very unstable when the spin ratio exceeds 2.
3) The main issue with the lifting line correction has been identified to be very strong tip velocities, which become unstable when combined with a full dynamic velocity field, as is the case in an actuator line simulation. The best found trick to counter this has been to apply some amount of tip loss smoothing the circulation distribution. This artificially smears the tip vortex over a larger part of the span, which greatly stabilizes the solution. However, it also reduces the lift-induced drag. Some of the reduction is likely partly physical when the goal is to model rotor sails with end disks. However, from experience, it seems the effect is too large for practical rotor configurations. As such, when using the tip loss factor, it is found to be necessary to add some amount of additional lift-induced drag to the model, through the *force correction* feature in the line force model.

## Why should interaction effects still be predicted OK?

Although the exact circulation distribution around a single rotor is likely not perfect with an actuator line, due to the corrections mentioned above, the current hypothesis is that this may not be that problematic when the goal is to predict interaction effects. The lift-induced velocities far away from a lifting surface is less sensitive to the exact shape then what it is close to it. That is, the overall lift may mater a lot, but the exact shape of the distribution is likely less important. As such, the a corrected model of a rotor sail may be sufficient when the goal is predict how it either interacts with other rotors or even how it interacts with the rest of the ship.

The main goal with the example in this folder is the check how close it is to the raw lifting line results. However, both are also compared against experimental data.

## Overview of scripts

- `rotor_setup.py`: Shared setup, used by both these examples and the OpenFOAM examples. It contains the actuator line model, the domain, the flow settings, the wind condition, and the post-processing of the forces. These are only defined here, so that both solvers use exactly the same settings.
- `plotting.py`: Storage of the results and plotting functionality, used by both these examples and the OpenFOAM examples. The simulation scripts only store the force histories as time series, and all plotting is done with this script. Use `--solver openfoam` to plot the OpenFOAM results.
- `run_single_rotor.py`: Runs a single rotor with Stormflow for different spin ratios and with different corrections applied.
- `mesh_convergence_test.py`: Mesh convergence test. It runs a single spin ratio (default 3.0) with increasing resolution at the rotor (default 4, 8, 16 and 32 cells per chord length), starting with the lowest.
- `run_two_rotors.py`: Runs two rotors, with the interfering rotor at different angular positions around the other, at a spin ratio of 2.5. This is the actuator line version of `two_rotors_interference_factors.py` in the lifting line example. By default, all the angular positions from the experiments by Deybach et al. (2026) are simulated, but a few can be chosen with `--angular-positions`, since the simulations take time. The interference factors are plotted with `python plotting.py interference`, together with the quasi-steady lifting line results, which are computed when plotting, and the experimental data.

The simulation domain is the wind tunnel from the experiments, scaled to full scale with the same factor as the rotor (model scale: rotor diameter 0.4 m, rotor height 2.4 m, foundation height 0.15 m, and a wind tunnel that is 10 m long, 5 m wide and 3 m high).

The same examples can be run with OpenFOAM, using the scripts in [`cfd_interfaces/openfoam/examples/rotor_sails/deybach_wind_tunnel`](../../../../cfd_interfaces/openfoam/examples/rotor_sails/deybach_wind_tunnel). Everything except the mesh is shared with the scripts in this folder.

All the simulation scripts only run the recommended variant by default. Use `--variant` to choose one or more other variants, or `--variant all` to run all of them.
