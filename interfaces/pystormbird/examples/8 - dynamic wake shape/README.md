# Dynamic wake shape

The default option in Stormbird is to **not** use the lift-induced velocities when updating the wake shape. This is mainly because of this taking a significantly longer time, while also *usually* not affecting the force predictions significantly.

However, there are situations where the full dynamic shape of the wake might influence the sail-to-sail interactions. In particular, this is found to be the case when one sail is standing close to another, and the apparent wind direction is such that the wake from one is almost directly hitting the other. 

This examples demonstrates how one can set up a simulation to compute the full dynamic shape of the wake. Two different solver modes will be used; one where the simulation is fully dynamic, and one where the strength is updated as if the simulation was steady-state. The latter alternative might be useful if you want to predict forces on the sails for steady-state applications, but have a situation where the dynamic shape is important. 