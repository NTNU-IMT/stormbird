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

from ..direct_setup.controller import ControllerBuilder, ControllerLogic, InternalStateType, SpinRatioConversion

import numpy as np

class SailType(Enum):
    WingSailSingleElement = "WingSailSingleElement"
    WingSailFlapped = "WingSailFlapped"
    RotorSail = "RotorSail"
    SuctionSail = "SuctionSail"

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
                section_model = SectionModel(model=Foil())
            case SailType.WingSailFlapped:
                internal_state_data = np.radians([-15.0, 0.0, 15.0])

                foils_data = [
                    Foil(cl_zero_angle = -1.75),
                    Foil(cl_zero_angle = 0.0),
                    Foil(cl_zero_angle = 1.75)
                ]

                section_model = SectionModel(model=VaryingFoil(
                    internal_state_data=internal_state_data,
                    foils_data=foils_data
                ))
            case SailType.RotorSail:
                section_model = SectionModel(model=RotatingCylinder())
            case SailType.SuctionSail:
                raise NotImplementedError("Suction sail not implemented yet")
            case _:
                raise ValueError("Unsupported sail type:", self.sail_type)

        non_zero_circulation_at_ends = (False, False)

        return WingBuilder(
            section_points=section_points,
            chord_vectors=chord_vectors,
            section_model=section_model,
            non_zero_circulation_at_ends=non_zero_circulation_at_ends
        )
    
    def controller_builder(self) -> ControllerBuilder:
        match self.sail_type:
            case SailType.WingSailSingleElement:
                apparent_wind_directions_data = np.radians([-180, -30, -15, 15, 30, 180])
                angle_of_attack_set_points_data = np.radians([-12.0, -12.0, 0.0, 0.0, 12, 12])

                logic = ControllerLogic(
                    apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
                    angle_of_attack_set_points_data = angle_of_attack_set_points_data.tolist()
                )

                return ControllerBuilder(
                    logic = logic
                )
            case SailType.WingSailFlapped:
                apparent_wind_directions_data = np.radians([-180, -30, -15, 15, 30, 180])
                angle_of_attack_set_points_data = np.radians([-12.0, -12.0, 0.0, 0.0, 12, 12])
                section_model_internal_state_set_points_data = np.radians([-15.0, -15.0, 0.0, 0.0, 15.0, 15.0])

                logic = ControllerLogic(
                    apparent_wind_directions_data =apparent_wind_directions_data.tolist(),
                    angle_of_attack_set_points_data=angle_of_attack_set_points_data.tolist(),
                    section_model_internal_state_set_points_data = section_model_internal_state_set_points_data.tolist()
                )

                return ControllerBuilder(
                    logic = logic
                )
            case SailType.RotorSail:
                apparent_wind_directions_data = np.radians([-180, -40, -30, 30, 40, 180])
                section_model_internal_state_set_points_data = [4.0, 4.0, 0.0, 0.0, -4.0, -4.0]

                internal_state_type = InternalStateType.SpinRatio
                internal_state_conversion = SpinRatioConversion(
                    diameter = self.chord_length,
                    max_rps = 180.0 / 60.0
                )

                logic = ControllerLogic(
                    apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
                    section_model_internal_state_set_points_data = section_model_internal_state_set_points_data,
                    internal_state_type = internal_state_type,
                    internal_state_conversion = internal_state_conversion
                )

                return ControllerBuilder(
                    logic = logic
                )
            case SailType.SuctionSail:
                raise NotImplementedError("Suction sail not implemented yet")
            case _:
                raise ValueError("Unsupported sail type:", self.sail_type)