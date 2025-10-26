# Line model representation of wings

The fundamental building block of all the methods in Stormbird is a simplified *line representation* of the lifting surfaces. This means that the full geometry of the wings is reduced to multiple discrete **line elements**. Each line element represents a section of a lifting surface that has the following properties:

- A **line segment geometry**, represented as a start point and and end point, defining the location and orientation of the element. The line segment also has *control point*, defined to be in the middle of the line segment. This point is used when computing local flow properties for the element during a simulation.
- A **chord vector**, which defines both the orientation and length of the chord. The orientation is relevant for computing the angle of attack as a function of the local velocity, while the length is relevant for computing the magnitude of the forces.
- A **sectional model** which is used to compute lift- and drag coefficients as a function of the local flow properties. The line model itself makes no assumptions about how the lift and drag is computed. However, Stormbird comes with a limited set of sectional models that are further described in [Sectional models](/sectional_models/sectional_models_intro.md). **Note**: in the actual implementation, the section model is not actually stored for each line element, because most of the time, many elements will share the same sectional model. However, when iterating over line elements in the code there is a functionality to retrieve the relevant sectional model for that line element.

## Structure overview
A view of source code that defines a line force model data structure can be seen below to illustrate what data is available. There are also multiple methods connected to this data structure not shown here. The construction of a line force model is generally not done manually, but rather by a [builder](./building_line_model.md).

```rust
pub struct LineForceModel {
    pub span_lines_local: Vec<SpanLine>,
    pub chord_vectors_local_not_rotated: Vec<SpatialVector>,
    pub chord_lengths: Vec<f64>,
    pub section_models: Vec<SectionModel>,
    pub wing_indices: Vec<Range<usize>>,
    pub non_zero_circulation_at_ends: Vec<[bool; 2]>,
    pub density: f64,
    pub circulation_correction: CirculationCorrection,
    pub angle_of_attack_correction: AngleOfAttackCorrection,
    pub output_coordinate_system: CoordinateSystem,
    pub rigid_body_motion: RigidBodyMotion,
    pub local_wing_angles: Vec<f64>,
    pub chord_vectors_local: Vec<SpatialVector>,
    pub chord_vectors_global: Vec<SpatialVector>,
    pub chord_vectors_global_at_span_points: Vec<SpatialVector>,
    pub span_lines_global: Vec<SpanLine>,
    pub span_points_global: Vec<SpatialVector>,
    pub ctrl_points_global: Vec<SpatialVector>,
    pub ctrl_point_spanwise_distance: Vec<f64>,
    pub ctrl_point_spanwise_distance_non_dimensional: Vec<f64>,
    pub ctrl_point_spanwise_distance_circulation_model: Vec<f64>,
    pub input_power_models: Vec<InputPowerModel>,
}
```

More details on each field can be found in the code [documentation](https://docs.rs/stormbird/0.7.0/stormbird/). The construction of a line force model is generally not done with the structure directly, but rather through a [builder](/line_model/building_line_model.md)
