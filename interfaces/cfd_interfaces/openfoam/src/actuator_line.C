// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#include "fvMesh.H"
#include "fvMatrix.H"
#include "addToRunTimeSelectionTable.H"
#include "volFields.H"

#include "actuator_line.hpp"

#include "cpp_actuator_line.hpp"
#include "floating_point_guard.hpp"

namespace Foam {
    namespace fv {
        defineTypeNameAndDebug(ActuatorLine, 0);

        #ifdef STORMBIRD_OPENFOAM_ESI
            addToRunTimeSelectionTable(option, ActuatorLine, dictionary);
        #else
            addToRunTimeSelectionTable(fvModel, ActuatorLine, dictionary);
        #endif
    }
}

// ---- Version specific interface to the base class ----------------------------------------------
#ifdef STORMBIRD_OPENFOAM_ESI

Foam::fv::ActuatorLine::ActuatorLine(
    const word& name,
    const word& modelType,
    const dictionary& dict,
    const fvMesh& mesh
): cellSetOption(name, modelType, dict, mesh) {
    coeffs_.readEntry("fields", fieldNames_);
    applied_.setSize(fieldNames_.size(), false);

    this->initialize(coeffs_);
}

const Foam::labelUList& Foam::fv::ActuatorLine::selected_cells() const {
    return this->cells();
}

void Foam::fv::ActuatorLine::addSup(
    fvMatrix<vector>& eqn,
    const label field
) {
    this->add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(
    const volScalarField& rho,
    fvMatrix<vector>& eqn,
    const label field
) {
    this->add(eqn.psi(), eqn);
}

void Foam::fv::ActuatorLine::addSup(
    const volScalarField& alpha,
    const volScalarField& rho,
    fvMatrix<vector>& eqn,
    const label field
) {
    this->add(eqn.psi(), eqn);
}

#else

Foam::fv::ActuatorLine::ActuatorLine(
    const word& name,
    const word& modelType,
    const fvMesh& mesh,
    const dictionary& dict
):
    fvModel(name, modelType, mesh, dict),
    zone_(mesh, coeffs(dict)),
    UName_(coeffs(dict).lookupOrDefault<word>("U", "U"))
{
    this->initialize(coeffs(dict));
}

const Foam::labelUList& Foam::fv::ActuatorLine::selected_cells() const {
    return this->zone_.zone();
}

Foam::wordList Foam::fv::ActuatorLine::addSupFields() const {
    return wordList(1, this->UName_);
}

void Foam::fv::ActuatorLine::addSup(
    const volVectorField& U,
    fvMatrix<vector>& eqn
) const {
    this->add(U, eqn);
}

void Foam::fv::ActuatorLine::addSup(
    const volScalarField& rho,
    const volVectorField& U,
    fvMatrix<vector>& eqn
) const {
    this->add(U, eqn);
}

void Foam::fv::ActuatorLine::addSup(
    const volScalarField& alpha,
    const volScalarField& rho,
    const volVectorField& U,
    fvMatrix<vector>& eqn
) const {
    this->add(U, eqn);
}

bool Foam::fv::ActuatorLine::movePoints() {
    this->zone_.movePoints();
    this->need_update = true;

    return true;
}

void Foam::fv::ActuatorLine::topoChange(const polyTopoChangeMap& map) {
    this->zone_.topoChange(map);
    this->need_update = true;
}

void Foam::fv::ActuatorLine::mapMesh(const polyMeshMap& map) {
    this->zone_.mapMesh(map);
    this->need_update = true;
}

void Foam::fv::ActuatorLine::distribute(const polyDistributionMap& map) {
    this->zone_.distribute(map);
    this->need_update = true;
}

bool Foam::fv::ActuatorLine::read(const dictionary& dict) {
    if (fvModel::read(dict)) {
        this->zone_.read(coeffs(dict));
        this->UName_ = coeffs(dict).lookupOrDefault<word>("U", "U");
        this->need_update = true;

        return true;
    }

    return false;
}

#endif

// ---- Shared implementation ---------------------------------------------------------------------
void Foam::fv::ActuatorLine::initialize(const dictionary& coeffs) {
    // Path to the Stormbird setup file. Environment variables and OpenFOAM tags such as <case> are
    // expanded, and relative paths are taken relative to the case directory.
    fileName setup_path = coeffs.lookupOrDefault<fileName>(
        "stormbirdSetupPath", "system/stormbird_actuator_line.json"
    );

    setup_path.expand();

    if (!setup_path.isAbsolute()) {
        setup_path = mesh().time().globalPath() / setup_path;
    }

    Info<< "    Reading Stormbird setup from " << setup_path << endl;

    {
        FloatingPointExceptionGuard fpe_guard;

        this->model = stormbird_interface::new_actuator_line_from_file(setup_path);
    }

    const word time_name = openfoam_compatibility::time_name(mesh().time());

    this->body_force_field = new volVectorField(
        IOobject(
            "bodyForce",
            time_name,
            mesh(),
            IOobject::NO_READ,
            IOobject::AUTO_WRITE
        ),
        mesh(),
        dimensionedVector("bodyForce", dimless, Zero)
    );

    this->body_force_field_weight = new volScalarField(
        IOobject(
            "bodyForceWeight",
            time_name,
            mesh(),
            IOobject::NO_READ,
            IOobject::AUTO_WRITE
        ),
        mesh(),
        dimensionedScalar("bodyForceWeight", dimless, 0.0)
    );
}

void Foam::fv::ActuatorLine::sync_line_force_model_state() const {
    // The controller only runs on the master processor, so the wing angles and internal states it
    // sets must be copied to the other processors
    int nr_wings = this->model->nr_wings();

    std::vector<double> local_wing_angles(nr_wings, 0.0);
    std::vector<double> internal_states(nr_wings, 0.0);

    if (Pstream::master()) {
        for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
            local_wing_angles[wing_index] = this->model->get_local_wing_angle(wing_index);
            internal_states[wing_index] = this->model->get_wing_internal_state(wing_index);
        }
    }

    // Sync the values between processors
    for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
        reduce(local_wing_angles[wing_index], sumOp<double>());
        reduce(internal_states[wing_index], sumOp<double>());
    }

    for (int wing_index = 0; wing_index < nr_wings; wing_index++) {
        this->model->set_local_wing_angle(wing_index, local_wing_angles[wing_index]);
        this->model->set_wing_internal_state(wing_index, internal_states[wing_index]);
    }
}

void Foam::fv::ActuatorLine::add(
    const volVectorField& velocity_field,
    fvMatrix<vector>& eqn
) const {
    // All calls to the Stormbird library happen inside this function, so floating point exception
    // trapping is disabled for the whole scope. See floating_point_guard.hpp for why.
    FloatingPointExceptionGuard fpe_guard;

    const scalarField& cell_volumes = mesh().V();
    double time_step = mesh().time().deltaTValue();
    double time = mesh().time().value();
    vectorField& equation_source = eqn.source();

    // Synchronize the line force model state across all processors
    this->sync_line_force_model_state();

    // Recalculate the projection and velocity sampling data if needed
    if (this->need_update) {
        this->set_projection_data();

        if (this->model->use_point_sampling()) {
            this->set_velocity_sampling_data_interpolation();
        } else {
            this->set_velocity_sampling_data_integral();
        }
    }

    const labelList& cell_ids = this->relevant_cells_for_projection;

    // Set the velocity field for the actuator line model
    if (this->model->use_point_sampling()) {
        this->set_interpolated_velocity(velocity_field);
    } else {
        this->set_integrated_weighted_velocity(velocity_field);
    }

    // Calculate the circulation
    this->model->do_step(time, time_step);

    // Apply the body force to the equation source
    forAll(cell_ids, i) {
        label cell_id = cell_ids[i];

        std::array<double, 3> cell_velocity = {
            velocity_field[cell_id][0],
            velocity_field[cell_id][1],
            velocity_field[cell_id][2]
        };

        label line_index = this->dominating_line_element_index_projection[i];

        std::array<double, 3> force_to_project = this->model->force_to_project(
            line_index,
            cell_velocity
        );

        double body_force_weight = this->body_force_field_weight[0][cell_id];

        vector body_force(Zero);

        body_force[0] = force_to_project[0] * body_force_weight * cell_volumes[cell_id];
        body_force[1] = force_to_project[1] * body_force_weight * cell_volumes[cell_id];
        body_force[2] = force_to_project[2] * body_force_weight * cell_volumes[cell_id];

        equation_source[cell_id] += body_force;

        this->body_force_field[0][cell_id] = body_force / cell_volumes[cell_id];
    }

    // Check if the model needs to be updated at the next time step
    this->need_update = false;
    if (Pstream::master()) {
        this->need_update = model->update_controller(time, time_step);

        this->need_update = false; // TODO: figure out how to use this...

        this->model->write_results("postProcessing");
    }
    reduce(this->need_update, orOp<bool>());
}
