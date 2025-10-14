# Single wing sail

The point of this example is to demonstrate how a single wing sail can be simulated using the lifting line functionality in Stormbird. 

The simulations are executed in three different modes, to compare the output against each other. This is to demonstrate how the results are affected by different simplifications and corrections.

## Overview of files
- `simulation.py` sets up the general simulation, with the possibility of varying settings in the simulation. This file is therefore the main example of how to set up a simulation of a single wing sail.
- `single_case.py` uses the functionality in `simulation.py` to run a single simulation using either a single element foil section at a specified angle of attack, or a multi-element foil section where the flap angle also can be adjusted.
- `single_element_multiple_angles.py` simulates a single element wing sail for multiple angles of attack, and compares the output to experimental data from the scientific literature.
- `multi_element_multiple_angles.py` simulates a multi-element wing sail for multiple angles of attacks. 

