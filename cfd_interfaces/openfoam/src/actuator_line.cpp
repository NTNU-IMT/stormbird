// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#include "fvMesh.H"
#include "fvMatrix.H"
#include "geometricOneField.H"
#include "addToRunTimeSelectionTable.H"
#include "volFields.H"
#include "interpolationCellPoint.H"

#include "OFstream.H"

#include "actuator_line.hpp"

#include "cpp_actuator_line.hpp"

namespace Foam {
    namespace fv {
        defineTypeNameAndDebug(ActuatorLine, 0);
        addToRunTimeSelectionTable(option, ActuatorLine, dictionary);
    }
}

// Constructor
Foam::fv::ActuatorLine::ActuatorLine(
    const word& name, 
    const word& modelType, 
    const dictionary& dict, 
    const fvMesh& mesh
): cellSetOption(name, modelType, dict, mesh) {

    coeffs_.readEntry("fields", fieldNames_);
    applied_.setSize(fieldNames_.size(), false);

    this->model = stormbird_interface::new_actuator_line_from_file("actuator_line.json");

    this->body_force_field = new volVectorField(
        IOobject(
            "bodyForce",
            mesh_.time().timeName(),
            mesh_,
            IOobject::NO_READ,
            IOobject::AUTO_WRITE
    ),
        mesh_,
        dimensionedVector("bodyForce", dimensionSet(0,0,0,0,0,0,0), vector::zero)
    );

    this->body_force_field_weight = new volScalarField(
        IOobject(
            "bodyForceWeight",
            mesh_.time().timeName(),
            mesh_,
            IOobject::NO_READ,
            IOobject::AUTO_WRITE
    ),
        mesh_,
        dimensionedScalar("bodyForceWeight", dimensionSet(0,0,0,0,0,0,0), 0.0)
    );
}

void Foam::fv::ActuatorLine::set_projection_data() {
    const vectorField& cell_centers = mesh_.C();
    
    const labelList& cell_ids = cells();

    this->relevant_cells_for_projection = labelList();

    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        std::array<double, 3> cell_center = {
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        double body_force_weight = this->model->summed_projection_weights_at_point(cell_center);

        if (body_force_weight > this->projection_limit) {
            this->relevant_cells_for_projection.append(cell_id);

            this->dominating_line_element_index_projection.append(
                this->model->dominating_line_element_index_at_point(cell_center)
            );

            this->body_force_field_weight[0][cell_id] = body_force_weight;
        } else {
            this->body_force_field_weight[0][cell_id] = 0.0;
        }
    }

    this->projection_data_is_set = true;
}

void Foam::fv::ActuatorLine::sync_line_force_model_state() {
    int nr_wings = this->model->nr_wings();

    std::vector<double> local_wing_angles;

    for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
        local_wing_angles.push_back(0.0);
    }
    
    if (Pstream::master()) {
        for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
            local_wing_angles[wing_index] = this->model->get_local_wing_angle(wing_index);
        }
    }

    // Sync the wing angles between processors
    for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
        reduce(local_wing_angles[wing_index], sumOp<double>());
    }

    for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
        this->model->set_local_wing_angle(wing_index, local_wing_angles[wing_index]);
    }
}

void Foam::fv::ActuatorLine::add(const volVectorField& velocity_field, fvMatrix<vector>& eqn)
{
    const vectorField& cell_centers = mesh_.C();
    const scalarField& cell_volumes = mesh_.V();
    double time_step = mesh_.time().deltaTValue();
    double time = mesh_.time().value();

    this->sync_line_force_model_state();

    if (!this->projection_data_is_set) {
        this->set_projection_data();
    }

    const labelList& cell_ids = this->relevant_cells_for_projection;
    
    vectorField& equation_source = eqn.source();

    if (this->use_integral_velocity_sampling) {
        this->set_integrated_weighted_velocity(velocity_field);
    } else {
        this->set_interpolated_velocity(velocity_field);
    }

    this->model->do_step(time_step, time);
    
    if (Pstream::master()) {
        this->model->write_results();
    }

    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        std::array<double, 3> cell_center = {
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        std::array<double, 3> body_force_sb = this->model->distributed_body_force_at_point(cell_center);

        vector body_force(vector::zero);

        body_force[0] = body_force_sb[0] * cell_volumes[cell_id];
        body_force[1] = body_force_sb[1] * cell_volumes[cell_id];
        body_force[2] = body_force_sb[2] * cell_volumes[cell_id];

        equation_source[cell_id] += body_force;

        this->body_force_field[0][cell_id] = body_force / cell_volumes[cell_id];
    }
}

void Foam::fv::ActuatorLine::addSup(fvMatrix<vector>& eqn, const label fieldi) {
    this->add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi) {
    this->add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(const volScalarField& alpha, const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi) {
    this->add(eqn.psi(), eqn);
}
