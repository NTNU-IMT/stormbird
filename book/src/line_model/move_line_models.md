# Move and modify a line model

In dynamic simulations, it will often be interesting to apply motion to the sails and dynamically change the angle of attack or internal state of a section model. This can, for instance, be done to simulate how the sails affect the seakeeping or maneuvering abilities of a ship or to allow the operational variables of the sails be dependent on the current wind conditions. This chapter specifies how to apply motion data to the sails during a dynamic simulation and how to modify the operational variables for each sail.

## Motion variables

The motion of a line force model is specified by the `rigid_body_motion` field, which is of the `RigidBodyMotion` type. This structure looks like the following:

```rust
pub struct RigidBodyMotion {
    pub translation: SpatialVector,
    pub rotation: SpatialVector,
    pub velocity_linear: SpatialVector,
    pub velocity_angular: SpatialVector,
    pub rotation_type: RotationType,
}
```

It specifiies translation and rotation, both in terms of the curren positions and in terms of the velocities at any given time. The position is important both for updating the wake shape and for determining local wind conditions on each line segment. When applying the motion variables, the rotation will be applied first, then the translation. That is, the rotation happens around a local coordinate system always. The order of the rotation can be set with the `rotation_type` field, but the default is rotation in x first, then y, and then z.

The motion velocities, `velocity_linear` and `velocity_angular`, are important for the calcualtion of forces. The forces on the each line segment is primarily dependent on the local velocity, local angle of attack, and the internal state of the sectional models. As such, when a motion is applied to the line force model, it is necessary to calculate the *felt velocity* and *felt acceleration* of each line segment, so that this can further be used as input to the force calculation functions. This can be calculated from the gloabl rotation and translation velocity vectors using methods in the `RigidBodyMotion` structure. As will be further highlighted [later](force_calculations.md), the forces are estimated from a `SectionalForceInput` structure, which have fields as shown in the code block below. The velocity and acceleration values calculated in the `SectionalForceInput` structure are dependent on the motion of the sails.

```rust
pub struct SectionalForcesInput {
    pub circulation_strength: Vec<f64>,
    pub velocity: Vec<SpatialVector>,
    pub angles_of_attack: Vec<f64>,
    pub acceleration: Vec<SpatialVector>,
    pub rotation_velocity: SpatialVector,
    pub coordinate_system: CoordinateSystem,
}
```

Each element in the vectors in the `SectionalForceInput` corresponds to a line element in the `LineForceModel`. Both the velocity and the acceleration is a combination of freestream conditions, induced velocities, and motion velocities. The motion velocities can either be automatically calcualted based on fintie difference and the time history of the motion or set manually if the variables are available from some other sources (e.g., a rigid body solver of a ship)

## Control variables

Two types of control variables exist for the sails in a `LineForceModel`.

The first is the `local_wing_angles`, which defines the rotation of the sails around its local axis. The local axis is defined as the axis of the first span line. If the sails is defined to be oriented in the z-direction as the span direction, a local wing angle value will then rotate all chord vectors around the z-axis.

The second value is the internal state of the section model for each wing. This value can represent different things, depending on the sail type and how it is modeled. Typical values are flap angles, rotational speeds, and suction rates.

## Updating the LineForceModel

When setting new values for either the motion variables or the control variables it is important to do it in way that also updates all dependent internal variables. For instance, many of the variables that define the geometry of the wings are defined both with their local values and their global values (e.g., `chord_vectors_local` and `chord_vectors_global`). In general, the global version of a variable type is calcualted from the local one, with the motion and wing angles applied to them. As such, updating the line force model should be done with `set` methods that also updates the global representation of the line force model.

The line force model have a general update function that should update all global geometry variables, and which should also be called after all `set` methods.

### Motions

Motion can be set in two ways: either with the position and rotation only, and using finite difference to calculate the corresponding velocities due to this motion, or by updating the velocity values manually. The first option is generally the easiest. However, the latter might be useful in cases where motion velocity is available from an external source, such as a rigid body solver that integrates the acceleration of an entire ship.

Below is some examples of how to apply different motion variables through the Python interface.

```python
import numpy as np

from pystormbird.lifting_line import Simulation

# ----- code to set up the simulation first, which includes the line force model -----
simulation = Simulation(setup_string)

translation = [1.0, 2.0, 3.0]
rotation = np.radians([10.0, 0.0, 0.0]).tolist()

time_step = 0.1

# Update the translation and rotation with the velocity set by finite difference
simulation.set_translation_and_rotation_with_finite_difference_for_the_velocity(
    time_step = time_step,
    translation = translation,
    rotation = rotation
)

# Alternativluy, update everything manually
simulation.set_translation_only(translation)
simulation.set_rotation_only(rotation)

linear_velocity = [8.0, 0.0, 0.0]
angular_velocity = np.radians([0.0, 10.0, 0.0]).tolist()

simulation.set_velocity_linear(linear_velocity)
simulation.set_velocity_angular(angular_velocity)

```



### Local chord angle control

Updating the chord angles in the Python interface is done by set-method that takes a list of angles as input, that must have a length equal to the number of wings in the line force model. An example code snippet is shown below, for a case with three sails:

```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_local_wing_angles(
    np.radians([40.0, 35.0, 30.0]).tolist()
)
```

### Update the internal state

The internal state of the section models can be updated with a set method, similar to how the local wing angles can be updated. This is shown in the code example below:


```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_section_models_internal_state(
    np.radians([10, 12.5, 15]).tolist()
)
```
