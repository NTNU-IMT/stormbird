// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#include "fvMesh.H"
#include "fvMatrix.H"
#include "geometricOneField.H"
#include "addToRunTimeSelectionTable.H"
#include "volFields.H"
#include "interpolationCellPoint.H"

#include "OFstream.H"

#include "actuator_line.hpp"

#include "cpp_actuator_line.hpp"

void Foam::fv::ActuatorLine::set_projection_data() {
    const vectorField& cell_centers = mesh_.C();
    
    const labelList& cell_ids = cells();

    this->relevant_cells_for_projection = labelList();

    double weight_limit = this->model->projection_weight_limit();

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
            this->relevant_cells_for_projection.append(cell_id);

            this->dominating_line_element_index_projection.append(
                dominating_line.line_index
            );

            this->body_force_field_weight[0][cell_id] = body_force_weight;
        } else {
            this->body_force_field_weight[0][cell_id] = 0.0;
        }
    }
}
