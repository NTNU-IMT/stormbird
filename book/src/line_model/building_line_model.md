# Building a line model

As shown in the [intro section](./line_model_intro.md), a line model consist of many line elements. To simplify the construction of this line model, Stormbird uses a *line force model builder* that helps with at least two things:

1) **Reduce the amount of input data:** Rather than having to specify data directly for each line element, it is possible to only specify data at some chosen points along the span - such as the beginning and the end - and let the builder interpolate for every line segment between the specified points
2) **Automate the setup of multiple wings:** The line force model requires information about which line element belongs to which wing. This can be cumbersome to set up manually, and it is not really necessary to do so. The builder automatically keep tracks of which line element belong to which wing.

## Input data
The Rust definition of the builder structure looks like the following:

```rust
pub struct LineForceModelBuilder {
    pub wing_builders: Vec<WingBuilder>,
    pub nr_sections: usize,
    pub density: f64,
    pub circulation_corrections: CirculationCorrection,
    pub ctrl_point_chord_factor: f64,
    pub output_coordinate_system: CoordinateSystem,
}
```

The only required input is the vector containing  `WingBuilder` structures and the `nr_sections` [^nr_sections_note]. The nr sections should be tested for each project, and will affect both the accuracy and the computational speed. Typical values range between 10-50. The `density` is set to the [standard air density for 15 degrees Celsius](https://en.wikipedia.org/wiki/Density_of_air) by default ( which = 1.225 kg / m^3). 

The `circulation_corrections` is an enum, where the default variant is `None`, and therefore not used by default. There will be more on the circulation corrections option [later](./circulation_strength.md). This is only used in special circumstances, for instance when a pure lifting line simulation might fail due to numerical issues. 

The `ctrl_point_chord_factor` specifies where the control point for induced velocities should be. By default, this value is zero, and the control point is located at the middle of each span line. However, it is possible to move it in the same direction as the chord vector, by specifying a ratio of the chord length. This is currently mostly an experimental feature, and not recommended to be used for anything else.

The `output_coordinate_system` is an enum that specifies how the forces and moments from the line force model should be calculated. The default is `Global`, which means all values are in a global coordinate system. That is, the coordinate system for the forces are not moved even if the wings are moved during a simulation. The other option is to set it to `Body`. In this case, the coordinate system of the forces will always follow the wings when they are moved.

[^nr_sections_note]: The number of sections for set in the `LineForceModelBuilder` is used as default, except when another value is defined in the `WingBuilder` below.

### Wing builder
A wing builder contain data to build line segments for a single wing. When a vector (or list) of wing builders are provided, the `LineForceModelBuilder` will automatically keep track of which line segment belong to each wing. The fields in a `WingBuilder` structure is as shown as Rust code below:

```rust
pub struct WingBuilder {
    pub section_points: Vec<SpatialVector<3>>,
    pub chord_vectors: Vec<SpatialVector<3>>,
    pub section_model: SectionModel,
    pub non_zero_circulation_at_ends: [bool; 2],
    pub nr_sections: Option<usize>,
}
```

The input consist of a set of `section_points` which contain information about the span line position. The minimum number of section points is two, and the first has to start at one end of the wing, while the last must end up at the other. There can, however, also be more section points in between the ends. 

For each section point, there also needs to be `chord_vectors`, which specify the local chord at the points. The `chord_vectors` give information about both chord length and orientation. 

Each wing also needs a [sectional model](./../sectional_models/sectional_models_intro.md). The sectional model can differ between different wings given to the `LineForceModelBuilder`, which is useful for cases where different sail types are installed on the same ship (uncommon today, but might happen in the future).

The `non_zero_circulation_at_ends` is boolean values that specifies the expected behavior of the circulation distribution at the ends of the wing. It is used when initializing the circulation distribution before a simulation, and when applying corrections to the circulation distribution. In most cases, the expected value of the circulation strength at the ends is zero, and the value of this variable should be `[false, false]`. However, if, for instance, the wing is standing directly on a symmetry plane, or is directly coupled to another wing, the circulation distribution will typically not be zero. If an end is coupled to *something* that might give a non-zero circulation, then that end should then get a `true` value. For instance, if the first end is standing on a symmetry plane, the value of this variable should be `[true, false]`.

The `nr_sections` variable is optional but can be set for each wing to override the default parameters in the `LineForceModelBuilder`. Typically, it will not be used, as the most common scenario is to have the same number of sections for each wing. 

When building a line force model from a wing builder, the wings are divided into multiple line segments based on the data in the `WingBuilder` structure. Each point in between the specified points are linearly interpolated [^interpolation_note]

[^interpolation_note]: The interpolation method is possible to change or update to something that can handle non-linear changes between section points. This is, however, so far not prioritized as most sail types tend to use fairly simple geometrical structures. This might change in the future if there is a need to do so.

## Input to methods
In many cases, the methods in Stormbird will handle the actual building of the line force models automatically, using a builder structure as input. That is, the input to some function is often the builder and not the line force model itself. For instance, when setting up a lifting line simulation in Python, you only have to supply the builder data, and the line force model will be built automatically internally.  

However, it is also possible to convert a builder into a line force model. This happens by calling the `build()` method, as usual for Rust builder structures. 

