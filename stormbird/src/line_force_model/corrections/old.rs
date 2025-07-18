pub fn smoothing_end_conditions(
        &self, 
        wing_index: usize, 
        value_type_to_be_smoothed: ValueTypeToBeSmoothed
    ) -> [EndCondition<f64>; 2] {
        let non_zero_circulation_at_ends = self.non_zero_circulation_at_ends[wing_index];

        let alpha_zero = match value_type_to_be_smoothed {
            ValueTypeToBeSmoothed::AngleOfAttack => {
                let section_model = &self.section_models[wing_index];

                match section_model {
                    SectionModel::Foil(foil) => -foil.cl_zero_angle / foil.cl_initial_slope,
                    SectionModel::VaryingFoil(foil) => {
                        let current_foil = foil.get_foil();

                        -current_foil.cl_zero_angle / current_foil.cl_initial_slope
                    },
                    SectionModel::RotatingCylinder(_) => 0.0
                }
            },
            ValueTypeToBeSmoothed::Circulation => 0.0
        };

        let mut end_conditions = [EndCondition::Extended, EndCondition::Extended];

        for i in 0..2 {
            end_conditions[i] = if non_zero_circulation_at_ends[i] {
                EndCondition::Extended
            } else {
                match value_type_to_be_smoothed {
                    ValueTypeToBeSmoothed::Circulation => EndCondition::Zero,
                    ValueTypeToBeSmoothed::AngleOfAttack => EndCondition::Given(alpha_zero)
                }
            };
        }
        
        end_conditions
    }

    /// Function that applies a Gaussian smoothing to the supplied strength vector.
    pub fn gaussian_smoothed_values(
        &self, 
        noisy_values: &[f64],
        settings: &GaussianSmoothing,
    ) -> Vec<f64> {
        if settings.length_factor <= 0.0 {
            return noisy_values.to_vec();
        }
        
        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        let wing_span_lengths = self.span_lengths();
        
        let span_distance = self.span_distance_in_local_coordinates();

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let smoothing_length = settings.length_factor * wing_span_lengths[wing_index];

            let end_conditions = self.smoothing_end_conditions(
                wing_index, value_type_to_be_smoothed
            );

            let local_span_distance = span_distance[wing_indices.clone()].to_vec();
            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let gaussian_smoothing = smoothing::gaussian::GaussianSmoothing {
                smoothing_length,
                number_of_end_insertions: None,
                end_conditions,
                delta_x_factor_end_insertions: 0.5,
                number_of_end_points_to_interpolate: settings.number_of_end_points_to_interpolate
            };

            let wing_smoothed_values = gaussian_smoothing.apply_smoothing(
                &local_span_distance, 
                &local_noisy_values, 
            );
            
            for index in 0..wing_smoothed_values.len() {
                smoothed_values.push(wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }

    pub fn polynomial_smoothed_values(
        &self, 
        noisy_values: &[f64]
    ) -> Vec<f64> {
        let mut smoothed_values: Vec<f64> = Vec::with_capacity(noisy_values.len());

        for (wing_index, wing_indices) in self.wing_indices.iter().enumerate() {
            let end_conditions = self.smoothing_end_conditions(
                wing_index, value_type_to_be_smoothed
            );

            let local_noisy_values = noisy_values[wing_indices.clone()].to_vec();

            let polynomial_smoothing = smoothing::polynomial::CubicPolynomialSmoothing {
                window_size: smoothing::polynomial::WindowSize::Seven,
                end_conditions: end_conditions
            };

            let wing_smoothed_values = polynomial_smoothing.apply_smoothing(&local_noisy_values);

            for index in 0..wing_smoothed_values.len() {
                smoothed_values.push(wing_smoothed_values[index]);
            }
        }

        smoothed_values
    }