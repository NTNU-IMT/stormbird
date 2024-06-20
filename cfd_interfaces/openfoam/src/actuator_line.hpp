// Copyright (C) 2024, NTNU 
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see seperate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

#ifndef ACTUATOR_LINE_H
#define ACTUATOR_LINE_H

#include <vector>
#include "cellSetOption.H"
#include "cpp_actuator_line.hpp"

namespace Foam {
    namespace fv {
        class ActuatorLine: public cellSetOption {
        public:
            TypeName("actuatorLine")

            stormbird_interface::CppActuatorLine* model;

            ActuatorLine(const word& name, const word& modelType, const dictionary& dict, const fvMesh& mesh);
            virtual ~ActuatorLine() = default;

            virtual void addSup(fvMatrix<vector>& eqn, const label fieldi);
            virtual void addSup(const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi);
            virtual void addSup(const volScalarField& alpha, const volScalarField& rho, fvMatrix<vector>& eqn, const label fieldi);

        private:
            volVectorField* body_force_field;
            volScalarField* body_force_field_weight;

            void add(const volVectorField& U, fvMatrix<vector>& eqn);
            void add_cell_information_to_model(const volVectorField& velocity);
            void add_interpolated_velocity(const volVectorField& velocity);
            
            double measure_average_cell_length(const std::vector<stormbird_interface::Vec3>& points);

            ActuatorLine(const ActuatorLine&) = delete;
            void operator=(const ActuatorLine&) = delete;
        };
    }
}

#endif
