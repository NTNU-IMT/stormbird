// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#ifndef ACTUATOR_LINE_H
#define ACTUATOR_LINE_H

#include "cellSetOption.H"
#include "cpp_actuator_line.hpp"

namespace Foam {
    namespace fv {
        class ActuatorLine: public cellSetOption {
        public:
            TypeName("actuatorLine")

            ActuatorLine(
                const word& name, 
                const word& modelType, 
                const dictionary& dict, 
                const fvMesh& mesh
            );

            virtual ~ActuatorLine() = default;

            virtual void addSup(
                fvMatrix<vector>& eqn, 
                const label fieldi
            );
            
            virtual void addSup(
                const volScalarField& rho, 
                fvMatrix<vector>& eqn, 
                const label fieldi
            );

            virtual void addSup(
                const volScalarField& alpha, 
                const volScalarField& rho, 
                fvMatrix<vector>& eqn, 
                const label fieldi
            );

        private:
            // Actuator line model
            stormbird_interface::CppActuatorLine* model;
            
            // Fields
            volVectorField* body_force_field;
            volScalarField* body_force_field_weight;

            // Mesh to actuator line data
            bool use_integral_velocity_sampling = false;
            
            bool velocity_sampling_data_is_set = false;
            bool projection_data_is_set = false;

            std::vector<label> interpolation_cells;
            std::vector<vector> ctrl_points;

            labelList relevant_cells;

            // Custom add function
            void add(const volVectorField& velocity, fvMatrix<vector>& eqn);

            // Check which cells are relevant
            void set_projection_data();
            void set_velocity_sampling_data_interpolation();

            // Ways to estimate the velocity
            void set_integrated_weighted_velocity(const volVectorField& velocity);
            void set_interpolated_velocity(const volVectorField& velocity);

            // Copy constructor and assignment operator
            ActuatorLine(const ActuatorLine&) = delete;
            void operator=(const ActuatorLine&) = delete;
        };
    }
}

#endif
