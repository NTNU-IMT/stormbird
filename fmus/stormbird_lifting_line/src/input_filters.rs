use stormath:: smoothing::moving_average::MovingAverage;

#[derive(Debug, Default, Clone)]
pub struct InputFilters {
    pub translation_x: MovingAverage,
    pub translation_y: MovingAverage,
    pub translation_z: MovingAverage,
    pub rotation_x: MovingAverage,
    pub rotation_y: MovingAverage,
    pub rotation_z: MovingAverage,
    pub motion_velocity_linear_x: MovingAverage,
    pub motion_velocity_linear_y: MovingAverage,
    pub motion_velocity_linear_z: MovingAverage,
    pub motion_velocity_angular_x: MovingAverage,
    pub motion_velocity_angular_y: MovingAverage,
    pub motion_velocity_angular_z: MovingAverage,
}

impl InputFilters {
    pub fn new(window_size: usize) -> Self {
        Self {
            translation_x: MovingAverage::new(window_size),
            translation_y: MovingAverage::new(window_size),
            translation_z: MovingAverage::new(window_size),
            rotation_x: MovingAverage::new(window_size),
            rotation_y: MovingAverage::new(window_size),
            rotation_z: MovingAverage::new(window_size),
            motion_velocity_linear_x: MovingAverage::new(window_size),
            motion_velocity_linear_y: MovingAverage::new(window_size),
            motion_velocity_linear_z: MovingAverage::new(window_size),
            motion_velocity_angular_x: MovingAverage::new(window_size),
            motion_velocity_angular_y: MovingAverage::new(window_size),
            motion_velocity_angular_z: MovingAverage::new(window_size),
        }
    }
}