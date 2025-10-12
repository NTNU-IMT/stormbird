
use stormath::type_aliases::Float;

#[derive(Debug, Clone)]
pub struct WakeIndices {
    pub nr_points_along_span: usize,
    pub nr_panels_along_span: usize,
    pub nr_panels_per_line_element: usize,
    pub nr_points_per_line_element: usize,
}

impl WakeIndices {
    #[inline(always)]
    /// Returns a flatten index for the wake panels. The panels are ordered streamwise-major.
    pub fn panel_index(&self, stream_index: usize, span_index: usize) -> usize {
        stream_index * self.nr_panels_along_span + span_index
    }

    #[inline(always)]
    /// Returns the stream and span indices from a flatten index
    pub fn reverse_panel_index(&self, flat_index: usize) -> (usize, usize) {
        let stream_index = flat_index / self.nr_panels_along_span;
        let span_index   = flat_index % self.nr_panels_along_span;

        (stream_index, span_index)
    }

    #[inline(always)]
    /// Returns a flatten index for the wake points. The points are ordered streamwise-major.
    pub fn point_index(&self, stream_index: usize, span_index: usize) -> usize {
        stream_index * self.nr_points_along_span + span_index
    }

    #[inline(always)]
    /// Return the total number of panels
    pub fn nr_panels(&self) -> usize {
        self.nr_panels_along_span * self.nr_panels_per_line_element
    }

    #[inline(always)]
    /// Return the total number of points
    pub fn nr_points(&self) -> usize {
        self.nr_points_along_span * self.nr_points_per_line_element
    }
}

#[derive(Debug, Clone)]
/// Settings for the wake
pub struct WakeSettings {
    /// The length of the first panel, relative to the local chord length
    pub first_panel_relative_length: Float,
    /// The length of the last panel, relative to the local chord length
    pub last_panel_relative_length: Float,
    /// A variable to determine of the chord direction should be used for the wake direction
    pub use_chord_direction: bool,
    /// A variable which panels that should be updated with the induced velocities included in the 
    /// velocity calculation
    pub end_index_induced_velocities_on_wake: usize,
    /// The amount of damping in the shape of the wake
    pub shape_damping_factor: Float,
    /// A variable to determine whether the self-induced velocities should be neglected or not
    pub neglect_self_induced_velocities: bool,
    /// A variable to determine whether the wake geometry and data should be written to a file
    pub write_wake_data_to_file: bool,
    /// The path to the folder where the wake data should be written to
    pub wake_files_folder_path: String,
}