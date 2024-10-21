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
    pub chord_vectors_local:  Vec<SpatialVector<3>>,
    pub section_models: Vec<SectionModel>,
    pub wing_indices:   Vec<Range<usize>>,
    pub translation: SpatialVector<3>,
    pub rotation: SpatialVector<3>,
    pub local_wing_angles: Vec<f64>,
    pub non_zero_circulation_at_ends: Vec<[bool; 2]>,
    pub density: f64,
    pub circulation_corrections: CirculationCorrection,
    pub ctrl_point_chord_factor: f64,
    pub output_coordinate_system: CoordinateSystem,
}
```

The `span_lines_local` and `chord_vectors_local` define the wing geometry in a local coordinate system. When the geometry is accessed in simulation methods, it is often the global geometry that is of interest. This is available through *getter methods* which apply the right transformations to the geometry. How to modify the geometry during a simulation is explained [here](./move_line_models.md). In short, it can be one by setting the `translation`, `rotation` and `local_wing_angles` fields.

Multiple line elements are combined to make up either a single lifting surface or many lifting surfaces together. There are no assumptions about the location and orientation of the line elements. They can, therefore, be oriented into any shape (although most wind propulsion configurations will consist of many straight wings...). Due to the nature of the simulation methods, each line element is treated as a a separate entity, where forces are only dependent on local flow conditions and the geometry and models connected to each element. However, when building up a single wing, it is generally natural that one line element lies next to the other line elements belonging to that wing. As such, the line elements can be grouped together to form a wing by defining a *first* and *last* element (also known as a `Range` in `Rust`) for a wing. This is managed by the `wing_indices` field. 

The `section_models` vector contain a section model for each wing in the line force model. 

The `density`, `non_zero_circulation_at_ends`, `ctrl_point_chord_factor`, `circulation_corrections` and `output_coordinate_system` is explained further in the [builder chapter](./building_line_model.md).
