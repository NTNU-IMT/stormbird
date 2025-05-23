use crate::line_force_model::LineForceModel;

fn elliptic_distribution(strength_0: f64, non_dim_span: f64) -> f64 {
    strength_0 * (1.0 - (2.0 * non_dim_span).powi(2)).sqrt()
}

impl LineForceModel {
    /// Returns a strength vector where the end values are corrected based on an assumption of an
    /// elliptical circulation distribution. This is to handle sitiations where the end values are
    /// noisy, while the interior values are not.
    pub fn apply_elliptic_end_correction_to_strength(
        &self,
        raw_strength: &[f64],
    ) -> Vec<f64> {
        let effective_span_distance = self.effective_span_distance_for_prescribed_circulations();

        let mut new_strength = raw_strength.to_vec();

        for wing_index in 0..self.nr_wings() {
            let first_line_index = self.wing_indices[wing_index].start;
            let second_line_index = first_line_index + 1;
            let last_line_index = self.wing_indices[wing_index].end - 1;
            let second_to_last_line_index = last_line_index - 1;

            let second_strength_value = raw_strength[second_line_index];
            let second_span_distance = effective_span_distance[second_line_index];

            let first_strength_0 = second_strength_value / elliptic_distribution(1.0, second_span_distance);

            new_strength[first_line_index] = elliptic_distribution(first_strength_0, effective_span_distance[first_line_index]);

            let second_to_last_strength_value = raw_strength[second_to_last_line_index];
            let second_to_last_span_distance = effective_span_distance[second_to_last_line_index];

            let last_strength_0 = second_to_last_strength_value / elliptic_distribution(1.0, second_to_last_span_distance);

            new_strength[last_line_index] = elliptic_distribution(last_strength_0, effective_span_distance[last_line_index]);
        }

        new_strength
    }
}