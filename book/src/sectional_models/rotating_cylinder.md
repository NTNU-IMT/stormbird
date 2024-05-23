# Rotating cylinder
Model representing a rotating cylinder. The lift, drag and moment can be calculated based on how fast the cylinder is spinning. 

## Available parameters
```rust
pub struct RotatingCylinder {
    /// The rotational speed of the rotor, in revolutions per second.
    pub revolutions_per_second: f64,
    /// Spin ratio data used when interpolating lift and drag coefficients.
    pub spin_ratio_data: Vec<f64>,
    /// Lift coefficient data as a function of spin ratio
    pub cl_data: Vec<f64>,
    /// Drag coefficient data as a function of spin ratio
    pub cd_data: Vec<f64>,
    /// The angle of the wake behind the cylinder, as a function of spin ratio.
    pub wake_angle_data: Option<Vec<f64>>,
    /// Added mass factor for the cylinder
    pub added_mass_factor: f64,
    /// Two-dimensional moment of inertia
    pub moment_of_inertia_2d: f64,
}
```