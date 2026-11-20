// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#include "fvMesh.H"
#include "volFields.H"
#include "interpolation.H"

#include "actuator_line.hpp"

#include "cpp_actuator_line.hpp"

void Foam::fv::ActuatorLine::set_velocity_sampling_data_integral() const {
    const vectorField& cell_centers = mesh().C();
    
    const labelUList& cell_ids = this->selected_cells();

    this->relevant_cells_for_velocity_sampling.clear();
    this->dominating_line_element_index_sampling.clear();

    this->relevant_cells_for_velocity_sampling = labelList();
    this->dominating_line_element_index_sampling = labelList();

    double weight_limit = this->model->sampling_weight_limit();

    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        std::array<double, 3> cell_center = {
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        auto dominating_line = this->model->dominating_line_element_and_weight_at_point(cell_center);

        double body_force_weight = dominating_line.weight;

        if (body_force_weight > weight_limit) {
            this->relevant_cells_for_velocity_sampling.append(cell_id);

            this->dominating_line_element_index_sampling.append(
                dominating_line.line_index
            );

            this->body_force_field_weight[0][cell_id] = body_force_weight;
        } else {
            this->body_force_field_weight[0][cell_id] = 0.0;
        }
    }
}

void Foam::fv::ActuatorLine::set_velocity_sampling_data_interpolation() const {
    this->ctrl_points.clear();
    this->interpolation_cells.clear();
    
    for (unsigned i = 0; i < this->model->nr_span_lines(); i++) {
        std::array<double, 3> point_sb = this->model->get_ctrl_point_at_index(i);
        
        this->ctrl_points.push_back(
            vector(point_sb[0], point_sb[1], point_sb[2])
        );

        this->interpolation_cells.push_back(
            openfoam_compatibility::find_cell(mesh(), this->ctrl_points[i])
        );
    }
}


// --------------------- Perform the interpolation -------------------------------------------------
void Foam::fv::ActuatorLine::set_integrated_weighted_velocity(const volVectorField& velocity_field) const {
    const vectorField& cell_centers = mesh().C();
    const scalarField& cell_volumes = mesh().V();

    // Initialize the numerator and denominator
    std::vector<vector> numerator;
    std::vector<double> denominator;

    int nr_span_lines = this->model->nr_span_lines();

    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        numerator.push_back(vector::zero);
        denominator.push_back(0.0);
    }

    // Loop over all cells for the current processor
    forAll(this->relevant_cells_for_velocity_sampling, i) {
        label cell_id = this->relevant_cells_for_velocity_sampling[i];

        std::array<double, 3> velocity = {
            velocity_field[cell_id][0],
            velocity_field[cell_id][1],
            velocity_field[cell_id][2]
        };

        double cell_volume = cell_volumes[cell_id];
       
        std::array<double, 3> cell_center = {
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        auto line_index = this->dominating_line_element_index_sampling[i];
        
        std::array<double, 4> temp_out = this->model->get_weighted_velocity_sampling_integral_terms_for_cell(
            line_index,
            velocity,
            cell_center,
            cell_volume
        );

        numerator[line_index][0] += temp_out[0];
        numerator[line_index][1] += temp_out[1];
        numerator[line_index][2] += temp_out[2];
        denominator[line_index] += temp_out[3];
    }

    // Sync the values between processors
    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        reduce(numerator[line_index], sumOp<vector>());
        reduce(denominator[line_index], sumOp<double>());
    }

    // Set the values in the model 
    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        if (denominator[line_index] != 0.0) {
            std::array<double, 3> velocity = {
                numerator[line_index][0] / denominator[line_index],
                numerator[line_index][1] / denominator[line_index],
                numerator[line_index][2] / denominator[line_index]
            };

            this->model->set_velocity_at_index(line_index, velocity);
        }
        
    }
}

void Foam::fv::ActuatorLine::set_interpolated_velocity(const volVectorField& velocity_field) const {    
    // Cell-point interpolation. Created through the run-time selection, as the class name differs
    // between the OpenFOAM versions.
    autoPtr<interpolation<vector>> u_interpolator = interpolation<vector>::New(
        "cellPoint", velocity_field
    );

    for (unsigned int i = 0; i < this->ctrl_points.size(); i++) {
        vector u_sample = vector(VGREAT, VGREAT, VGREAT);

        label cell_id = this->interpolation_cells[i];

        if (this->interpolation_cells[i] != -1) {
            u_sample = u_interpolator->interpolate(this->ctrl_points[i], cell_id);
        }
        
        reduce(u_sample, minOp<vector>());

        std::array<double, 3> velocity = {
            u_sample[0],
            u_sample[1],
            u_sample[2]
        };

        this->model->set_velocity_at_index(i, velocity);
    }
 }

void Foam::fv::ActuatorLine::set_wake_points_velocity(const volVectorField& velocity_field) const {
    const label nr_points = this->model->nr_wake_points_to_sample();

    if (nr_points == 0) {
        return;
    }

    // The points move in each time step, so the cells are searched for every time
    autoPtr<interpolation<vector>> u_interpolator = interpolation<vector>::New(
        "cellPoint", velocity_field
    );

    // The sum of the velocity from the processors that found each point, and the number of such
    // processors, packed into one field, so that all points are synced with one reduce
    scalarField packed_values(4 * nr_points, 0.0);

    for (label i = 0; i < nr_points; i++) {
        std::array<double, 3> point_sb = this->model->get_wake_point_at_index(i);

        const point wake_point(point_sb[0], point_sb[1], point_sb[2]);

        const label cell_id = openfoam_compatibility::find_cell(mesh(), wake_point);

        if (cell_id != -1) {
            const vector u_sample = u_interpolator->interpolate(wake_point, cell_id);

            packed_values[4 * i] += u_sample[0];
            packed_values[4 * i + 1] += u_sample[1];
            packed_values[4 * i + 2] += u_sample[2];
            packed_values[4 * i + 3] += 1.0;
        }
    }

    reduce(packed_values, sumOp<scalarField>());

    // Points that are not found on any processor, i.e., outside the domain, are not set, so that
    // the model keeps the previous value
    for (label i = 0; i < nr_points; i++) {
        const scalar nr_found = packed_values[4 * i + 3];

        if (nr_found > 0.0) {
            std::array<double, 3> velocity = {
                packed_values[4 * i] / nr_found,
                packed_values[4 * i + 1] / nr_found,
                packed_values[4 * i + 2] / nr_found
            };

            this->model->set_wake_point_velocity_at_index(i, velocity);
        }
    }
}
