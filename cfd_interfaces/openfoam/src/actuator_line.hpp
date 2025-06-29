// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

///
/// This is the OpenFOAM interface for the actuator line model in Stormbird. The interface consists
/// of a C++ class that inherits from the necessary base class in OpenFOAM to implement volume 
/// forces which holds the actuator line model in Stormbird as a member. 
/// 

#ifndef ACTUATOR_LINE_H
#define ACTUATOR_LINE_H

#include "cellSetOption.H"
#include "cpp_actuator_line.hpp"

namespace Foam {
    namespace fv {
        class ActuatorLine: public cellSetOption {
        public:
            TypeName("actuatorLine")

            /// Constructor
            ActuatorLine(
                const word& name, 
                const word& modelType, 
                const dictionary& dict, 
                const fvMesh& mesh
            );

            /// Destructor
            virtual ~ActuatorLine() = default;

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

        private:
            /// The Stormbird actuator line model
            stormbird_interface::CppActuatorLine* model;
            
            /// The body force field, that will be exported by OpenFOAM during the simulation
            volVectorField* body_force_field;
            /// The body force field weight, that will be exported by OpenFOAM during the simulation
            volScalarField* body_force_field_weight;

            // Parameters for the sampling and projection
            // TODO: make these parameters available in the input file
            bool use_integral_velocity_sampling = true;
            bool only_use_dominating_line_element_when_sampling = true;
            bool only_use_dominating_line_element_when_projecting = true;
            double projection_limit = 0.001;
            double sampling_integral_limit = 0.001;

            // Switch to determine if the OpenFOAM data needs to be updated, due to changes in the 
            // actuator line model
            bool need_update = true;
            
            // Store all relevant data
            std::vector<label> interpolation_cells;
            std::vector<vector> ctrl_points;

            labelList relevant_cells_for_projection;
            labelList dominating_line_element_index_projection;

            labelList relevant_cells_for_velocity_sampling;
            labelList dominating_line_element_index_sampling;

            /// The add function, intended to be use across all the OpenFOAM addSup functions
            void add(const volVectorField& velocity, fvMatrix<vector>& eqn);

            // Check which cells are relevant
            void set_projection_data();
            void set_velocity_sampling_data_interpolation();
            void set_velocity_sampling_data_integral();

            /// Ways to estimate the velocity
            void set_integrated_weighted_velocity(const volVectorField& velocity);
            void set_interpolated_velocity(const volVectorField& velocity);

            /// This method is used to synchronize the line force model state across all the 
            /// instances across the processors
            void sync_line_force_model_state();

            // Copy constructor and assignment operator
            ActuatorLine(const ActuatorLine&) = delete;
            void operator=(const ActuatorLine&) = delete;
        };
    }
}

#endif
