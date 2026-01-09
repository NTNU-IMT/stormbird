'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from enum import Enum

from ..base_model import StormbirdSetupBaseModel
from ..direct_setup.spatial_vector import SpatialVector
from ..direct_setup.line_force_model import WingBuilder
from ..direct_setup.section_models import SectionModel, Foil, VaryingFoil, RotatingCylinder
from ..direct_setup.controller import ControllerBuilder, ControllerLogic
from ..direct_setup.input_power import InputPowerModel

import numpy as np

class SailType(Enum):
    WingSailSingleElement = "WingSailSingleElement"
    WingSailFlapped = "WingSailFlapped"
    RotorSail = "RotorSail"
    SuctionSail = "SuctionSail"

    def consumes_power(self) -> bool:
        match self:
            case SailType.RotorSail | SailType.SuctionSail:
                return True
            case SailType.WingSailSingleElement | SailType.WingSailFlapped:
                return False
            case _:
                raise ValueError("Unsupported sail type:", self)

class SimpleSailSetup(StormbirdSetupBaseModel):
    '''
    Class that is used to quickly set up models for different sail types. It stores typical settings
    for the different sail types, which are used to create the wing builder and controller builder.
    '''
    position: SpatialVector = SpatialVector()
    chord_length: float
    height: float
    sail_type: SailType

    def wing_builder(self) -> WingBuilder:
        section_points = [
            self.position,
            SpatialVector(
                x=self.position.x,
                y=self.position.y,
                z=self.position.z + self.height
            )
        ]

        chord_vectors = [
            SpatialVector(x=self.chord_length, y=0.0, z=0.0),
            SpatialVector(x=self.chord_length, y=0.0, z=0.0)
        ]

        match self.sail_type:
            case SailType.WingSailSingleElement:
                section_model = SectionModel(model=Foil(
                    cd_min = 0.01,
                ))
            case SailType.WingSailFlapped:
                internal_state_data = np.radians([-15.0, 0.0, 15.0]).tolist()

                foils_data = [
                    Foil(cl_zero_angle = -1.75, cd_min = 0.01),
                    Foil(cl_zero_angle = 0.0, cd_min = 0.01),
                    Foil(cl_zero_angle = 1.75, cd_min = 0.01)
                ]

                section_model = SectionModel(model=VaryingFoil(
                    internal_state_data = internal_state_data,
                    foils_data=foils_data
                ))
            case SailType.RotorSail:
                section_model = SectionModel(model=RotatingCylinder())
            case SailType.SuctionSail:
                raise NotImplementedError("Suction sail not implemented yet")
            case _:
                raise ValueError("Unsupported sail type:", self.sail_type)

        non_zero_circulation_at_ends = (False, False)

        wing_builder =  WingBuilder(
            section_points=section_points,
            chord_vectors=chord_vectors,
            section_model=section_model,
            non_zero_circulation_at_ends=non_zero_circulation_at_ends
        )

        match self.sail_type:
            case SailType.RotorSail:
                # Simple input power model based on public data from Norse power.
                diameter_data = np.array([4.0, 5.0])
                max_rps_data = np.array([255.0, 180.0]) / 60.0
                nominal_power_data = np.array([100.0, 175.0]) * 1000.0

                input_power_model = InputPowerModel.new_polynomial_rotor_sail_model(
                    max_power = np.interp(self.chord_length, diameter_data, nominal_power_data),
                    max_rps = np.interp(self.chord_length, diameter_data, max_rps_data),
                    area = self.chord_length * self.height
                )

                wing_builder.input_power_model = input_power_model
            case SailType.SuctionSail:
                raise NotImplementedError("Suction sail not implemented yet")
            case _:
                pass

        return wing_builder

    def controller_builder(self) -> ControllerBuilder:
        match self.sail_type:
            case SailType.WingSailSingleElement:
                apparent_wind_directions_data = np.radians([-180, -30, -10, 10, 30, 180])
                angle_of_attack_set_points_data = np.radians([-15.0, -15.0, 0.0, 0.0, 15, 15])

                logic = ControllerLogic(
                    apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
                    angle_of_attack_set_points_data = angle_of_attack_set_points_data.tolist()
                )

                return ControllerBuilder(
                    logic = logic
                )
            case SailType.WingSailFlapped:
                apparent_wind_directions_data = np.radians([-180, -30, -10, 10, 30, 180])
                angle_of_attack_set_points_data = np.radians([-12.0, -12.0, 0.0, 0.0, 12, 12])
                section_model_internal_state_set_points_data = np.radians([-15.0, -15.0, 0.0, 0.0, 15.0, 15.0])

                logic = ControllerLogic(
                    apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
                    angle_of_attack_set_points_data = angle_of_attack_set_points_data.tolist(),
                    section_model_internal_state_set_points_data = section_model_internal_state_set_points_data.tolist()
                )

                return ControllerBuilder(
                    logic = logic
                )
            case SailType.RotorSail:
                return ControllerBuilder.new_default_rotor_sail(
                    diameter = self.chord_length, 
                    max_rps = 180.0 / 60.0
                )
            case SailType.SuctionSail:
                raise NotImplementedError("Suction sail not implemented yet")
            case _:
                raise ValueError("Unsupported sail type:", self.sail_type)
