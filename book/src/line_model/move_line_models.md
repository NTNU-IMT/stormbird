# Move and modify a line model

In dynamic simulations, it will often be interesting to apply motion to the sails, or perhaps dynamically change the angle of attack or internal state of a section model. This can, for instance, be done to simulate how the sails affect the seakeeping or maneuvering abilities of a ship or to allow the operational variables of the sails be dependent on the current wind conditions. This chapter specifies how to apply motion data to the sails during a dynamic simulation and how to modify the operational variables for each sail.

## How motion affects the simulation results

As mentioned in the [force calculation chapter](force_calculations.md), the forces on the each line segment is primarily dependent on the local velocity, local angle of attack and internal state of the sectional models. However, some forces will also be generated due to the rotational velocity of the chord vectors and the acceleration of each line segment.

When applying motion to a line force model, it is necessary to calculate the *felt velocity* and *fel acceleration* of each line segment, so that this can further be used as input to the force calculation functions. As will be further highlighted [later](force_calculations.md), the forces are estimated from data a `SectionalForceInput` structure, which have fields as shown in the code block below. The velocity and acceleration values calculated in the `SectionalForceInput` structure are dependent on the motion of the sails.

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

Each element in the vectors in the `SectionalForceInput` corresponds to a line element in the `LineForceModel`. Both the velocity and the acceleration is a combination of freestream conditions, induced velocities, and motion velocities.

## Collective motion

The sails can be moved collectively by setting the `translation` and `rotation` vectors.  

## Local control

