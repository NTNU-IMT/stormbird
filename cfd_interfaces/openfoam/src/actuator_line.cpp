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

void Foam::fv::ActuatorLine::add_cell_information_to_model(const volVectorField& velocity) {
    const vectorField& cell_centers = mesh_.C();
    const scalarField& cell_volumes = mesh_.V();
    const labelList& cell_ids       = cells();

    this->model->clear_cell_information();

    // -------------------- Loop over all cells for the current processor --------------------------
    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        auto center_sb = stormbird_interface::Vec3{
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        auto velocity_sb = stormbird_interface::Vec3{
            velocity[cell_id][0],
            velocity[cell_id][1],
            velocity[cell_id][2]
        };

        double cell_volume = cell_volumes[cell_id];

        this->model->add_cell_information(center_sb, velocity_sb, cell_volume);
    }

    // ------------------ Sync the values between processors ---------------------------------------

    int n = this->model->nr_sampling_span_lines();

    for (int i = 0; i < n; i++) {
        auto numerator_sb = this->model->get_velocity_sampling_numerator(i);

        vector numerator(vector::zero);
        
        numerator[0] = numerator_sb.x;
        numerator[1] = numerator_sb.y;
        numerator[2] = numerator_sb.z;

        reduce(numerator, sumOp<vector>());

        numerator_sb.x = numerator[0];
        numerator_sb.y = numerator[1];
        numerator_sb.z = numerator[2];

        this->model->set_velocity_sampling_numerator(i, numerator_sb);

        double denominator = this->model->get_velocity_sampling_denominator(i);

        reduce(denominator, sumOp<double>());

        this->model->set_velocity_sampling_denominator(i, denominator);
    }
}

void Foam::fv::ActuatorLine::add(const volVectorField& U, fvMatrix<vector>& eqn)
{
    const vectorField& cell_centers = mesh_.C();
    const scalarField& cell_volumes = mesh_.V();
    const labelList& cell_ids      = cells();
    vectorField& equation_source   = eqn.source();

    this->add_cell_information_to_model(U);

    this->model->calculate_result();
    
    if (Pstream::master()) {
        this->model->write_results();
    }

    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        auto cell_center_sb = stormbird_interface::Vec3{
            cell_centers[cell_id][0],
            cell_centers[cell_id][1],
            cell_centers[cell_id][2]
        };

        auto body_force_sb = this->model->distributed_body_force_at_point(cell_center_sb);

        double body_force_weight = this->model->distributed_body_force_weight_at_point(cell_center_sb);

        vector body_force(vector::zero);

        body_force[0] = body_force_sb.x * cell_volumes[cell_id];
        body_force[1] = body_force_sb.y * cell_volumes[cell_id];
        body_force[2] = body_force_sb.z * cell_volumes[cell_id];

        equation_source[cell_id] += body_force;

        this->body_force_field[0][cell_id] = body_force / cell_volumes[cell_id];
        this->body_force_field_weight[0][cell_id] = body_force_weight;
    }
}

double Foam::fv::ActuatorLine::measure_average_cell_length(const std::vector<stormbird_interface::Vec3>& points) {
    double volume_sum = 0.0;
    
    for (unsigned int i = 0; i < points.size(); i++) {
        scalar volume_sample = scalar(VGREAT);

        vector point_local = vector(points[i].x, points[i].y, points[i].z);

        label cell_id = mesh_.findCell(point_local);

        if (cell_id != -1) {
            volume_sample = mesh_.V()[cell_id];
        }
        
        reduce(volume_sample, minOp<scalar>());

        volume_sum += volume_sample; 
    }

    double average_volume = volume_sum / points.size();
    double average_cell_length = pow(average_volume, 1.0/3.0);

    return average_cell_length;
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
