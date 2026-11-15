// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

///
/// This is the OpenFOAM interface for the actuator line model in Stormbird. The interface consists
/// of a C++ class that inherits from the necessary base class in OpenFOAM to implement volume
/// forces which holds the actuator line model in Stormbird as a member.
///
/// Both the ESI/Keysight version (openfoam.com) and the Foundation version (openfoam.org) are
/// supported. They use different base classes for volume forces:
///
/// - ESI: fv::cellSetOption, set up in system/fvOptions
/// - Foundation: fvModel, set up in constant/fvModels
///
/// Only the interface to the base class differs between the versions. The actual work is done in
/// shared functions, which are const so that they can be called from the const addSup functions in
/// the Foundation version. The data that is updated during the simulation is therefore mutable.
///

#ifndef ACTUATOR_LINE_H
#define ACTUATOR_LINE_H

#include "openfoam_compatibility.hpp"

// The version specific headers are included through macros, because the dependency scanner in
// wmake ignores #ifdef, and warns about headers that only exist in the other version. It only
// follows includes of literal file names, so the headers below are skipped by the scanner.
#ifdef STORMBIRD_OPENFOAM_ESI
    #define STORMBIRD_BASE_CLASS_HEADER "cellSetOption.H"
    #include STORMBIRD_BASE_CLASS_HEADER
#else
    #define STORMBIRD_BASE_CLASS_HEADER "fvModel.H"
    #define STORMBIRD_CELL_ZONE_HEADER "fvCellZone.H"
    #include STORMBIRD_BASE_CLASS_HEADER
    #include STORMBIRD_CELL_ZONE_HEADER
#endif

#include "volFields.H"
#include "fvMatrix.H"
#include <vector>
#include <array>

#include "cpp_actuator_line.hpp"

namespace Foam {
    namespace fv {
        #ifdef STORMBIRD_OPENFOAM_ESI
            using ActuatorLineBase = cellSetOption;
        #else
            using ActuatorLineBase = fvModel;
        #endif

        class ActuatorLine: public ActuatorLineBase {
        public:
            TypeName("actuatorLine")

        // ---- Version specific interface to the base class ----------------------------------
        #ifdef STORMBIRD_OPENFOAM_ESI
            /// Constructor
            ActuatorLine(
                const word& name,
                const word& modelType,
                const dictionary& dict,
                const fvMesh& mesh
            );

            /// Necessary function in OpenFOAM to add volume forces in solvers which do not use the
            /// density in the momentum equation
            virtual void addSup(
                fvMatrix<vector>& eqn,
                const label fieldi
            );

            /// Necessary function in OpenFOAM to add volume forces in solvers which use the density
            /// in the momentum equation
            virtual void addSup(
                const volScalarField& rho,
                fvMatrix<vector>& eqn,
                const label fieldi
            );

            /// Necessary function in OpenFOAM to add volume forces in solvers which use the density
            /// and the volume of fluid in the momentum equation
            virtual void addSup(
                const volScalarField& alpha,
                const volScalarField& rho,
                fvMatrix<vector>& eqn,
                const label fieldi
            );
        #else
            /// Constructor
            ActuatorLine(
                const word& name,
                const word& modelType,
                const fvMesh& mesh,
                const dictionary& dict
            );

            /// The fields the model adds a source to, i.e., the velocity
            virtual wordList addSupFields() const;

            /// Adds volume forces in solvers which do not use the density in the momentum equation
            virtual void addSup(
                const volVectorField& U,
                fvMatrix<vector>& eqn
            ) const;

            /// Adds volume forces in solvers which use the density in the momentum equation
            virtual void addSup(
                const volScalarField& rho,
                const volVectorField& U,
                fvMatrix<vector>& eqn
            ) const;

            /// Adds volume forces in solvers which use the density and the volume of fluid in the
            /// momentum equation
            virtual void addSup(
                const volScalarField& alpha,
                const volScalarField& rho,
                const volVectorField& U,
                fvMatrix<vector>& eqn
            ) const;

            // Mesh changes. The cell selection is updated, and the projection and sampling data
            // is recalculated at the next call to addSup.
            virtual bool movePoints();
            virtual void topoChange(const polyTopoChangeMap& map);
            virtual void mapMesh(const polyMeshMap& map);
            virtual void distribute(const polyDistributionMap& map);

            /// Re-reads the settings in the fvModels dictionary
            virtual bool read(const dictionary& dict);
        #endif

            /// Destructor
            virtual ~ActuatorLine() = default;

        private:
        #ifdef STORMBIRD_OPENFOAM_FOUNDATION
            /// The cells the model is applied to
            fvCellZone zone_;

            /// Name of the velocity field
            word UName_;
        #endif

            /// The Stormbird actuator line model
            stormbird_interface::CppActuatorLine* model;

            /// The body force field, that will be exported by OpenFOAM during the simulation
            volVectorField* body_force_field;
            /// The body force field weight, that will be exported by OpenFOAM during the simulation
            volScalarField* body_force_field_weight;

            // Switch to determine if the OpenFOAM data needs to be updated, due to changes in the
            // actuator line model
            mutable bool need_update = true;

            // Store all relevant data
            mutable std::vector<label> interpolation_cells;
            mutable std::vector<vector> ctrl_points;

            mutable labelList relevant_cells_for_projection;
            mutable labelList dominating_line_element_index_projection;

            mutable labelList relevant_cells_for_velocity_sampling;
            mutable labelList dominating_line_element_index_sampling;

            /// Shared part of the constructors: creates the Stormbird model and the output fields
            void initialize(const dictionary& coeffs);

            /// The cells selected for the model
            const labelUList& selected_cells() const;

            /// The add function, intended to be use across all the OpenFOAM addSup functions
            void add(const volVectorField& velocity, fvMatrix<vector>& eqn) const;

            // Check which cells are relevant
            void set_projection_data() const;
            void set_velocity_sampling_data_interpolation() const;
            void set_velocity_sampling_data_integral() const;

            /// Ways to estimate the velocity
            void set_integrated_weighted_velocity(const volVectorField& velocity) const;
            void set_interpolated_velocity(const volVectorField& velocity) const;

            void sync_line_force_model_state() const;

            // Copy constructor and assignment operator
            ActuatorLine(const ActuatorLine&) = delete;
            void operator=(const ActuatorLine&) = delete;
        };
    }
}

#endif
