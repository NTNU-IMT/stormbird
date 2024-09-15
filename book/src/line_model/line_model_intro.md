# Line model representation of wings

The fundamental building block of all the methods in Stormbird is a simplified *line representation* of the lifting surfaces. This means that the full geometry of the wings is reduced to multiple discrete **line elements**. Each line element represents a section of a lifting surface that has the following properties:

- A **line segment geometry**, represented as a start point and and end point, defining the location and orientation of the element. The line segment also has *control point*, defined to be in the middle of the line segment. This point is used when computing local flow properties for the element during a simulation.
- A **chord vector**, which defines both the orientation and length of the chord. The orientation is relevant for computing the angle of attack as a function of the local velocity, while the length is relevant for computing the magnitude of the forces.
- A **sectional model** which is used to compute lift- and drag coefficients as a function of the local flow properties. The line model itself makes no assumptions about how the lift and drag is computed. However, Stormbird comes with a limited set of sectional models that are further described in [Sectional models](/sectional_models/sectional_models_intro.md). **Note**: in the actual implementation, the section model is not actually stored for each line element, because most of the time, many elements will share the same sectional model. However, when iterating over line elements in the code there is a functionality to retrieve the relevant sectional model for that line element.

A view of source code that defines a line force model data structure can be seen below to illustrate what data is available. There are also multiple methods connected to this data structure not shown here.

```rust
pub struct LineForceModel {
    /// Vector of line segments that defines the span geometry of the wings. Each have its own start 
    /// and end point, to allow for uncoupled analysis
    pub span_lines_local: Vec<SpanLine>,
    /// Vectors representing both the chord length and the direction of the chord for each span line
    pub chord_vectors_local:  Vec<SpatialVector<3>>,
    /// Two dimensional models for lift and drag coefficients for each wing in the model
    pub section_models: Vec<SectionModel>,
    /// Indices used to sort different wings from each other.
    pub wing_indices:   Vec<Range<usize>>,
    /// Translation from local to global coordinates
    pub translation: SpatialVector<3>,
    /// Rotation from local to global coordinates
    pub rotation: SpatialVector<3>,
    /// Vector used to store local angles for each wing. This can be used to rotate the wing along 
    /// the span axis during a dynamic simulation. The typical example is changing the angle of 
    /// attack on a wing sail due to changing apparent wind conditions.
    pub local_wing_angles: Vec<f64>,
    /// A vector that contains booleans that indicate whether the circulation should be zero at the
    /// ends or not. The variables are used both when initializing the circulation before a 
    /// simulation and in cases where smoothing is applied to the circulation.
    /// The vector is structured as follows:
    /// - The first index is the wing index
    /// - The second index is the end index, where 0 means that start of the wind and 1 means the
    /// end
    /// - When the boolean is false, the circulation is set to zero at the end, and when it is true,
    ///  the circulation is assumed to be non-zero.
    pub non_zero_circulation_at_ends: Vec<[bool; 2]>,
    /// A vector containing information about whether or not a wing is 'virtual' or real. Virtual 
    /// wings are included in the simulations like non-virtual wings, except that the forces are not
    /// included in the total force calculations. The primary use case is modelling end plates. 
    /// Adding a virtual wing at the tip of another wing will reduce the tip losses, similar to how
    /// an end plate would work in reality.
    pub virtual_wings: Vec<bool>,
    /// Density used in force calculations
    pub density: f64,
    /// Optional model for calculation motion and flow derivatives
    pub derivatives: Option<Derivatives>,
    /// Optional corrections that can be applied to the estimated circulation strength.
    pub circulation_corrections: CirculationCorrection,
    /// Optional variables to 
    /// Factor used to control the control point location
    pub ctrl_point_chord_factor: f64,
}
```

Multiple line elements are combined to make up either a single lifting surface or many lifting surfaces together. There are no assumptions about the location and orientation of the line elements. They can, therefore, be oriented into any shape (although most wind propulsion configurations will consist of many straight wings...). Due to the nature of the simulation methods, each line element is treated as a a separate entity, where forces are only dependent on local flow conditions and the geometry and models connected to each element. However, when building up a single wing, it is generally natural that one line element lies next to the other line elements belonging to that wing. As such, the line elements can be grouped together to form a wing by defining a *first* and *last* element (also known as a `Range` in `Rust`) for a wing.

The line elements are first defined in a local coordinate system. However, when performing dynamic simulations is is often interesting to simulate moving wings.This is done by setting new positions and rotations for the lifting surfaces during each time step. The corresponding velocities and accelerations that occur due to updated motion parameters are automatically calculated in a simulation using a stored history of the parameters and finite difference schemes. 