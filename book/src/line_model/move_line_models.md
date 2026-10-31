# Move and modify a line model

In dynamic simulations, it will often be interesting to apply motion to the sails and dynamically change the angle of attack or the internal state of the wings. This can, for instance, be done to simulate how the sails affect the seakeeping or maneuvering abilities of a ship or to allow the operational variables of the sails to be dependent on the current wind conditions. This chapter specifies how to apply motion data to the sails during a dynamic simulation and how to modify the operational variables for each sail.

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

It specifies translation and rotation, both in terms of the current positions and in terms of the velocities at any given time. The position is important both for updating the wake shape and for determining local wind conditions on each line segment. When applying the motion variables, the rotation will be applied first, then the translation. That is, the rotation happens around a local coordinate system always. The order of the rotation can be set with the `rotation_type` field, but the default is rotation in x first, then y, and then z.

The motion velocities, `velocity_linear` and `velocity_angular`, are important for the calculation of forces. The forces on each line segment are primarily dependent on the local velocity, local angle of attack, and the internal state of the sectional models. As such, when a motion is applied to the line force model, it is necessary to calculate the *felt velocity* and *felt acceleration* of each line segment, so that this can further be used as input to the force calculation functions. This can be calculated from the global rotation and translation velocity vectors using methods in the `RigidBodyMotion` structure. As will be further highlighted [later](force_calculations.md), the forces are estimated from a `SectionalForcesInput` structure, which has fields as shown in the code block below. The velocity and acceleration values calculated in the `SectionalForcesInput` structure are dependent on the motion of the sails.

```rust
pub struct SectionalForcesInput {
    pub circulation_strength: Vec<f64>,
    pub velocity: Vec<SpatialVector>,
    pub angles_of_attack: Vec<f64>,
    pub section_models_internal_state: Vec<f64>,
    pub amount_of_stall: Vec<f64>,
    pub acceleration: Vec<SpatialVector>,
    pub rotation_velocity: SpatialVector,
    pub coordinate_system: CoordinateSystem,
}
```

Each element in the vectors in the `SectionalForcesInput` corresponds to a line element in the `LineForceModel`. Both the velocity and the acceleration are a combination of freestream conditions, induced velocities, and motion velocities. 

The `section_models_internal_state` is the internal state given to the section model on each line element, which is converted from the internal state set on the wing (see the [control variables](#control-variables) below). For some section models, the conversion depends on the local velocity, which means that the motion can also affect the section internal states. For instance, the spin ratio of a rotor sail with a constant rotational speed changes when the felt velocity changes, and the suction coefficient of a suction sail with a constant fan power varies with the local flow conditions. The `amount_of_stall` is the amount of stall used for the lift and drag on each line element, between 0 (no stall) and 1 (full stall), as explained in the [foil model chapter](../sectional_models/foil_model.md). For wings with a [dynamic stall model](../sectional_models/dynamic_stall.md), this is the dynamic amount of stall. The value is zero for section models that do not build on the foil model, such as the rotating cylinder.

The motion velocities can either be automatically calculated based on finite difference and the time history of the motion or set manually if the variables are available from some other sources (e.g., a rigid body solver of a ship).

## Control variables

Two types of control variables exist for the sails in a `LineForceModel`.

The first is the `local_wing_angles`, which defines the rotation of the sails around its local axis. The local axis is defined as the axis of the first span line. If the sail is defined to be oriented in the z-direction as the span direction, a local wing angle value will then rotate all chord vectors around the z-axis.

The second is the internal state of each wing. This value can represent different things, depending on the sail type and how it is modeled. Typical values are flap angles, rotational speeds, fan power, and suction coefficients. The internal state of the wing is converted to the internal state of the section model on each line element, which can be different. For instance, the internal state of a rotor sail wing is the revolutions per second, while the section model uses the spin ratio. The internal state can also be constant along the span, which is the most common, or vary along the span. See the [section model chapter](../sectional_models/sectional_models_intro.md#internal-states) for more on what the internal state means for each section model, and how it varies along the span.

## Updating the LineForceModel

When setting new values for either the motion variables or the control variables it is important to do it in a way that also updates all dependent internal variables. For instance, many of the variables that define the geometry of the wings are defined both with their local values and their global values (e.g., `chord_vectors_local` and `chord_vectors_global`). In general, the global version of a variable type is calculated from the local one, with the motion and wing angles applied to them. As such, updating the line force model should be done with `set` methods that also update the global representation of the line force model.

The line force model has a general update function that should update all global geometry variables, and which should also be called after all `set` methods.

### Motions

Motion can be set in two ways: either with the position and rotation only, and using finite difference to calculate the corresponding velocities due to this motion, or by updating the velocity values manually. The first option is generally the easiest. However, the latter might be useful in cases where motion velocity is available from an external source, such as a rigid body solver that integrates the acceleration of an entire ship.

Below are some examples of how to apply different motion variables through the Python interface.

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

# Alternatively, update everything manually
simulation.set_translation_only(translation)
simulation.set_rotation_only(rotation)

linear_velocity = [8.0, 0.0, 0.0]
angular_velocity = np.radians([0.0, 10.0, 0.0]).tolist()

simulation.set_velocity_linear(linear_velocity)
simulation.set_velocity_angular(angular_velocity)

```



### Local chord angle control

Updating the chord angles in the Python interface is done by a set-method that takes a list of angles as input, that must have a length equal to the number of wings in the line force model. An example code snippet is shown below, for a case with three sails:

```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_local_wing_angles(
    np.radians([40.0, 35.0, 30.0]).tolist()
)
```

### Update the internal state

The internal state of the wings can be updated with a set method, similar to how the local wing angles can be updated. The input is a list with one value per wing, which sets an internal state that is constant along the span of each wing. This is the most common case, and is shown in the code example below for three sails with a flap angle as the internal state:

```python
import numpy as np

# some code to set up the simulation, as above

simulation.set_internal_state(
    np.radians([10, 12.5, 15]).tolist()
)
```

An internal state that varies along the span is set for one wing at a time. There are two functions for this, corresponding to the two ways of giving a span variation. The locations along the span are non-dimensional, running from -0.5 at one end of the wing to 0.5 at the other:

```python
# A flap angle that varies linearly from 10 degrees at one end of the first wing to 20 degrees at
# the other end
simulation.set_internal_state_linear(
    wing_index = 0,
    locations = [-0.5, 0.5],
    values = np.radians([10.0, 20.0]).tolist()
)

# A suction sail with two internal compartments, with the boundary in the middle of the span. 
# Using the same boundaries as the compartments gives one value per compartment, which, for 
# instance, can be the rotational speed of the fan in each compartment
simulation.set_internal_state_piecewise_constant(
    wing_index = 1,
    boundaries = [-0.5, 0.0, 0.5],
    values = [15.0, 20.0]
)
```

The internal compartments in the last example are part of the geometry of the wing, and are set when building the line force model, through the `internal_compartment_boundaries` in the [wing builder](building_line_model.md). They can not be changed during a simulation. See the [active air flow foil chapter](../sectional_models/active_air_flow_foil.md#internal-compartments) for more on how the internal state is applied to each compartment.

The current internal state of each wing can be read with the `internal_state` function, which returns one value per wing. For a wing with an internal state that varies along the span, the value is the mean over the span.

In Rust, the corresponding functions on the `LineForceModel` are `set_internal_state`, for constant values for all wings, and `set_wing_internal_state` or `set_internal_state_span_variation`, for setting the internal state of a single wing as a `WingInternalState` or a `SpanVariation`. Unlike the motion and the local wing angles, the internal state does not affect the geometry of the wings, so no update of the global geometry is needed after setting it.

**Note**: if a [controller](../utils/control_system.md) is used, it sets a constant internal state for all wings each time it updates, which overwrites internal states that are set manually, including those that vary along the span.
