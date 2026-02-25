# Wind environment
DESCRIPTION TO BE EXTENDED LATER. AlSO, THE EXACT SHAPE OF THE DATA STRUCTURE STILL UNDER DEVELOPMENT.

The Stormbird library contains a data structure for representing the wind environment in different situations. The main point of this structure is to be able to provide spatially varying inflow conditions at all relevant points in a simulation. This fits most naturally with the lifting line simulations, but the wind model is kept separate so that it can, potentially, also be used with other simplified models of wind propulsion devices if needed.

The basic structure for the wind environment is given below:

```rust
pub struct WindEnvironment {
    pub height_variation_model: Option<HeightVariationModel>,
    pub up_direction: SpatialVector,
    pub wind_rotation_axis: SpatialVector,
    pub zero_direction_vector: SpatialVector,
    pub water_plane_height: f64,
    pub inflow_corrections: Option<InflowCorrections>,
}
```

The fields in the structure are described below:

- `height_variation_model`: An optional model for how the wind speed varies with height above the water surface. If this is not specified, the wind speed is assumed to be constant with height.
- `up_direction`: A vector defining the "up" direction in the simulation. This is used to determine the height above the water surface.
- `wind_rotation_axis`: A vector defining the axis where the wind direction is rotated around when changing wind direction.
- `zero_direction_vector`: A vector defining the reference direction for the wind. The wind will point along this vector when the wind direction angle is zero.
- `water_plane_height`: A scalar defining the height of the water plane in the simulation.
- `inflow_corrections`: An optional structure containing corrections to be applied to the inflow velocity, which is primarily intended for modeling disturbances due to the rest of the ship. See more about this correction model below.

## Height variation models
The height variation models can be set with the following Enum:

```rust
pub enum HeightVariationModel {
    PowerModel(PowerModel),
    LogarithmicModel(LogarithmicModel),
}
```

That is, one can choose between a power law model and a logarithmic model for the wind speed variation with height. The input structures for the two models are shown below:

```rust
pub struct PowerModel {
    pub reference_height: f64,
    pub power_factor: f64,
}

pub struct LogarithmicModel {
    pub reference_height: f64,
    pub surface_roughness: f64,
}
```

## Inflow corrections
NOTE: These structures, in particular, have room for improvements. Might therefore change in future versions. MORE TO COME LATER.

The `inflow_corrections` variable is a data structure used to store corrections to the freestream velocity due to anything that might disturb the inflow. That is, it is primarily intended to model the wind disturbances caused by the ship itself. The data structure is set up such that the correction can depend on both the spanwise position along each sail and the apparent wind direction.

The correction factors themselves consist of two parts: A magnitude factor that scales the local inflow velocity, and an angle correction that modifies the local apparent wind angle.

The data structures to set this up is shown below. In practice, these structures are just used to do linear interpolation on the correction factors as needed during the simulation.:

```rust
pub struct InflowCorrectionsSingleDirection {
    pub height_values: Vec<Float>,
    pub magnitude_corrections: Vec<Float>,
    pub angle_corrections: Vec<Float>, 
    pub wing_indices: Vec<Range<usize>>,
}

pub struct InflowCorrections {
    pub apparent_wind_directions: Vec<Float>,
    pub corrections: Vec<InflowCorrectionsSingleDirection>,
}
```

### Tuning the inflow corrections
TO COME
