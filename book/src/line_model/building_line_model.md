# Building a line model

As shown in the [intro section](./line_model_intro.md), a line model consist of many line elements. To simplify the construction of this line model, Stormbird uses a *line force model builder* that helps with at least two things:

1) **Reduce the amount of input data:** Rather than having to specify data directly for each line element, it is possible to only specify data at some chosen points along the span - such as the beginning and the end - at let the builder interpolate for every line segment between the specified points
2) **Automate the setup of multiple wings:** The line force model requires information about which line element belongs to which wing. This can be cumbersome to set up manually, and it is not really necessary to do so. The builder automatically keep tracks of which line element belong to which wing.

## Input data
The Rust definition of the builder structure looks like the following:

```rust
pub struct LineForceModelBuilder {
    pub wing_builders: Vec<WingBuilder>,
    pub nr_sections: usize,
    pub density: f64,
    pub prescribed_circulation: Option<PrescribedCirculation>,
}
```

The only required input is the vector of `WingBuilder`s and the `nr_sections` [^nr_sections_note]. The density is set to the air density by default ( which = 1.225 kg / m^3), and the prescribed circulation is not used by default [^prescribed_note]. The nr sections should be tested for each project, adn will affect both the accuracy and the computational speed. Typical values range between 10-50.

[^nr_sections_note]: Right now, the number of sections will be the same for each wing in the simulation. The interface was made in this way, because this scenario is believed to be the most common. However, it is fairly straight forward to implement functionality to allow each wing to have different values. This might be implemented in the future.

[^prescribed_note]: There will be more on the prescribed circulation option later. This is only used in special circumstances, for instance when a pure lifting line simulation might fail due to numerical issues. 

### Wing builders
The wing builders contain data to build line segments for a single wing. When a vector (or list) of wing builders are provided, the `LineForceModelBuilder` will automatically keep track of which line segment belong to each wing. The input to a wing builder is as shown as Rust code below:

```rust
pub struct WingBuilder {
    pub section_points: Vec<Vec3>,
    pub chord_vectors: Vec<Vec3>,
    pub section_model: SectionModel,
}
```

The input consist of a set of `section_points` which contain information about the span line position. The minimum number of section points is two, and the first has to start at one end of the wing, while the last must end up at the other. There can, however, also be more section points in between the ends. For each section point, there also needs to be `chord_vectors`, which specify the local chord at the points. The `chord_vectors` give information about both chord length and orientation. Each wing also needs a [sectional model](./../sectional_models/sectional_models_intro.md). The sectional model can differ between different wings given to the `LineForceModelBuilder`, which is useful for cases where different sail types are installed on the same ship (uncommon today, but might happen in the future).

When building a line force model from a wing builder, the wings are divided into multiple line segments based on the data in the `WingBuilder` structure. Each point in between the specified points are linearly interpolated [^interpolation_note]

[^interpolation_note]: The interpolation method is possible to change or update to something that can handle non-linear changes between section points. This is, however, so far not prioritized as most sail types tend to use fairly simple geometrical structures. This might change in the future if there is a need to do so.

## Input to methods
In many cases, the methods in Stormbird will handle the actual building of the line force models automatically, using a builder structure as input. That is, the input to some function is often the builder and not the line force model itself. For instance, when setting up a lifting line simulation in Python, you only have to supply the builder data, and the line force model will be built automatically internally.  

However, it is also possible to convert a builder into a line force model. This happens by calling the `build()` method, as usual for Rust builder structures. 

