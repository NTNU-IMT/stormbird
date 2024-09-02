# Simulation of wing sails close to stall

The point of this example is to illustrate how wing sails can be simulated when the angle of attack is so large that either parts of the sail or the complete sail is stalling. The challenge with this case is that the lift is both very large - causing large lift-induced velocities - and very non-linear - causing challenges for the numerical solver. 

To handle these challenges it is necessary to apply certain amounts of *numerical tricks*, mainly consisting of some damping of the estimated circulation distribution.

The actual simulation setup is found in `simulation.py`, which contains a function that execute a simulation based on some input. 

The scripts `single_cases.py` and `multiple_angles.py` then uses the function to run a single simulation or simulation of multiple angle of attack, and then plot the results in various ways. 

See also the *Stormbird book* for more discussions about this case.