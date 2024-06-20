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

void Foam::fv::ActuatorLine::add(const volVectorField& velocity_field, fvMatrix<vector>& eqn)
{
    const vectorField& cell_centers = mesh_.C();
    const scalarField& cell_volumes = mesh_.V();
    const labelList& cell_ids      = cells();
    vectorField& equation_source   = eqn.source();

    this->set_integrated_weighted_velocity(velocity_field);

    this->model->calculate_result();
    
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

        double body_force_weight = this->model->distributed_body_force_weight_at_point(cell_center);

        vector body_force(vector::zero);

        body_force[0] = body_force_sb[0] * cell_volumes[cell_id];
        body_force[1] = body_force_sb[1] * cell_volumes[cell_id];
        body_force[2] = body_force_sb[2] * cell_volumes[cell_id];

        equation_source[cell_id] += body_force;

        this->body_force_field[0][cell_id] = body_force / cell_volumes[cell_id];
        this->body_force_field_weight[0][cell_id] = body_force_weight;
    }
}

void Foam::fv::ActuatorLine::set_integrated_weighted_velocity(const volVectorField& velocity_field) {
    const vectorField& cell_centers = mesh_.C();
    const scalarField& cell_volumes = mesh_.V();
    const labelList& cell_ids       = cells();

    // ------------------ Initialize the numerator and denominator ---------------------------------
    std::vector<vector> numerator;
    std::vector<double> denominator;

    int nr_span_lines = this->model->nr_span_lines();

    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        numerator.push_back(vector::zero);
        denominator.push_back(0.0);
    }

    // -------------------- Loop over all cells for the current processor --------------------------
    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

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

        for (int line_index = 0; line_index < nr_span_lines; line_index++) {
            std::array<double, 4> temp_out = this->model->get_weighted_velocity_integral_terms_for_cell(
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
    }

    // ------------------ Sync the values between processors ---------------------------------------
    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        reduce(numerator[line_index], sumOp<vector>());
        reduce(denominator[line_index], sumOp<double>());
    }

    // ------------------ Set the values in the model ------------------------------------------------
    for (int line_index = 0; line_index < nr_span_lines; line_index++) {
        std::array<double, 3> velocity = {
            numerator[line_index][0] / denominator[line_index],
            numerator[line_index][1] / denominator[line_index],
            numerator[line_index][2] / denominator[line_index]
        };

        this->model->set_velocity_at_index(line_index, velocity);
    }
}

void Foam::fv::ActuatorLine::set_interpolated_velocity(const volVectorField& velocity_field) {
    std::vector<vector> points;

    for (unsigned i = 0; i < this->model->nr_span_lines(); i++) {
        std::array<double, 3> point_sb = this->model->get_ctrl_point_at_index(i);
        
        points.push_back(
            vector(point_sb[0], point_sb[1], point_sb[2])
        );
    }
    
    interpolationCellPoint<vector> u_interpolator(velocity_field); // create interpolation object

    for (unsigned int i = 0; i < points.size(); i++) {
        vector u_sample = vector(VGREAT, VGREAT, VGREAT);

        label cell_id = mesh_.findCell(points[i]);

        if (cell_id != -1) {
            u_sample = u_interpolator.interpolate(points[i], cell_id);
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

void Foam::fv::ActuatorLine::addSup(fvMatrix<vector>& eqn, const label fieldi) {
    add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi) {
    add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(const volScalarField& alpha, const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi) {
    add(eqn.psi(), eqn);
}
