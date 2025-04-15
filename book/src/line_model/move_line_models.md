# Move and modify a line model

In dynamic simulations, it will often be interesting to apply motion to the sails, or perhaps dynamically change the angle of attack or internal state of a section model. This can, for instance, be done to simulate how the sails affect the seakeeping or maneuvering abilities of a ship or to allow the operational variables of the sails be dependent on the current wind conditions. This chapter specifies how to apply motion data to the sails during a dynamic simulation and how to modify the operational variables for each sail.

## How motion affects the simulation results

The forces on the each line segment is primarily dependent on the local velocity, local angle of attack, and the internal state of the sectional models. Some forces will also be generated due to the rotational velocity of the chord vectors and the acceleration of each line segment.

As such, when a motion is applied to the line force model, it is necessary to calculate the *felt velocity* and *felt acceleration* of each line segment, so that this can further be used as input to the force calculation functions. As will be further highlighted [later](force_calculations.md), the forces are estimated from a `SectionalForceInput` structure, which have fields as shown in the code block below. The velocity and acceleration values calculated in the `SectionalForceInput` structure are dependent on the motion of the sails. 

```rust
pub struct SectionalForcesInput {
    pub circulation_strength: Vec<f64>,
    pub velocity: Vec<SpatialVector<3>>,
    pub angles_of_attack: Vec<f64>,
    pub acceleration: Vec<SpatialVector<3>>,
    pub angles_of_attack_derivative: Vec<f64>,
    pub rotation_velocity: SpatialVector<3>,
}
```

Each element in the vectors in the `SectionalForceInput` corresponds to a line element in the `LineForceModel`. Both the velocity and the acceleration is a combination of freestream conditions, induced velocities, and motion velocities. The motion velocities are automatically calculated by the line force model using a finite difference scheme based on the motion history [^fdm_note]. 

[^fdm_note]: The finite difference approach was chosen as this simplifies the necessary input in a dynamic simulation. However, the velocity and acceleration are in some cases also directly available, for instance, from a method that calculates the motion of a ship. Future extensions of the motion functionality ma therefore be to allow for more direct specification of the motion velocity and acceleration.

## Collective motion

The entire geometry of a line force model can be moved by setting the `rotation` and `translation` vector. The rotation is specified as rotation around the x-axis, y-axis and z-axis respectively, where the rotation operations are performed in the same order: x first, then y, then z. The rotation of the geometry will happen first, then the translation. The rotation is therefore specified as in how the geometry is rotated from the initial configuration.

In the Python interface, both vectors must be updated by using setting methods for the main [simulation class](../lifting_line/simulation_overview.md), as shown in the code snippet below:

```python
from pystormbird.lifting_line import Simulation
from pystormbird import SpatialVector

# Some code to generate a setup string for a lifting line simulation

simulation = Simulation(
    setup_string = json.dumps(setup),
    initial_time_step = dt,
    initialization_velocity = freestream_velocity
)

simulation.set_rotation(
    SpatialVector(x_rotation, y_rotation, z_rotation)
)

simulation.set_translation(
    SpatialVector(x_translation, y_translation, z_translation)
)
```

## Local chord angle control

The chord vectors can also be rotated independent of the rest of the geometry, with the axis of the wings as the rotation axis. This is typically done to change the angle of the sails, for instance to account for changes in the wind conditions. The interface allow for individual rotation for each wing. The rotation of the chord vectors happens before the collective rotation above, and must therefore be thought of as rotation in a body fixed coordinate system. In the Python interface, this is done by set-method that takes a list of angles as input, that must have a length equal to the number of wings in the line force model. An example code snippet is shown below, for a case with three sails:

```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_local_wing_angles(
    [np.radians(30), np.radians(35), np.radians(40)]
)
```

## Update the internal state

Some of the [section models](../sectional_models/sectional_models_intro.md) have an *internal state* that modifies how the model reacts to varying velocity. This can, for instance, be the rotational speed of a rotor sail, the suction rate of a suction sail, or the flap angle of a multi-element foil. The internal state of the section models can be updated with a set method, similar to how the local wing angles can be updated. This is shown in the code example below:


```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_section_models_internal_state(
    [np.radians(10), np.radians(12.5), np.radians(15)]
)
```
