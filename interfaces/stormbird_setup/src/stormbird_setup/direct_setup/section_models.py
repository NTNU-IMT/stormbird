'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ..base_model import StormbirdSetupBaseModel

from pydantic import model_serializer, model_validator

import numpy as np

class Foil(StormbirdSetupBaseModel):
    cl_zero_angle: float | None = None
    cl_initial_slope: float | None = None
    cl_high_order_factor_positive: float | None = None
    cl_high_order_factor_negative: float | None = None
    cl_high_order_power: float | None = None
    cl_max_after_stall: float | None = None
    cd_min: float | None = None
    angle_cd_min: float | None = None
    cd_second_order_factor: float | None = None
    cd_max_after_stall: float | None = None
    cd_power_after_stall: float | None = None
    cdi_correction_factor: float | None = None
    mean_positive_stall_angle: float | None = None
    mean_negative_stall_angle: float | None = None
    stall_range: float | None = None
    cd_stall_angle_offset: float | None = None
    cd_bump_during_stall: float | None = None
    added_mass_factor: float | None = None

class VaryingFoil(StormbirdSetupBaseModel):
    internal_state_data: list[float]
    foils_data: list[Foil]
    current_internal_state: float | None = None

class RotatingCylinder(StormbirdSetupBaseModel):
    revolutions_per_second: float  = 0.0
    spin_ratio_data: list[float] | None = None
    cl_data: list[float]| None = None
    cd_data: list[float] | None = None
    added_mass_factor: float | None = None
    
class EffectiveWindSensor(StormbirdSetupBaseModel):
    pass

class SectionModel(StormbirdSetupBaseModel):
    model: Foil | VaryingFoil | RotatingCylinder | EffectiveWindSensor
    
    @classmethod
    def default_rotor_sail(cls) -> "SectionModel":
        """
        Default model for a rotor sail, which also equals the default model for the RotatingCylinder
        model
        """
        return cls(
            model = RotatingCylinder()
        )
    
    @classmethod
    def default_wing_sail_single_element(cls) -> "SectionModel":
        """
        Default model for a single element wing sail, which also equals the default model for the 
        Foil model
        """
        return cls(
            model = Foil()
        )
        
    @classmethod
    def default_wing_sail_two_element(cls) -> "SectionModel":
        """
        Default values for a two-element wing sail.
        """
        internal_state_data = np.radians([-30.0, 0.0, 30.0]).tolist()

        foils_data = [
            Foil(cl_zero_angle = -2.0, cd_min = 0.01),
            Foil(cl_zero_angle = 0.0,  cd_min = 0.01),
            Foil(cl_zero_angle = 2.0,  cd_min = 0.01)
        ]

        return cls(
            model=VaryingFoil(
                internal_state_data = internal_state_data,
                foils_data=foils_data,
                current_internal_state = 0.0
            )
        )
    
    @classmethod
    def default_suction_sail(cls) -> "SectionModel":
        """
        Default values from a suction sail, manually tuned so that they roughly represent the 
        results presented in:
        https://www.jmwe.org/uploads/1/0/6/4/106473271/aa_suction_sails_turbosail_ventifoil_cousteau_report.pdf
        """
        
        # The power coefficient values used to represent the intenral state
        ca_values = [0.0, 0.1187, 0.2161, 0.3389]
        
        cl_zero_angle = [0.0, 2.6, 3.4, 3.8]
        cl_initial_slope = [2 * np.pi, 2*np.pi * 1.4, 2*np.pi * 1.4, 2*np.pi * 1.4]
        stall_angles = np.radians([20.0, 23, 27, 29.0]).tolist()
        
        # Make the model valid for both positive and negative Ca values
        ca_values_full = [-x for x in ca_values[1::-1]] + ca_values
        cl_zero_angle_full = [-x for x in cl_zero_angle[1::-1]] + cl_zero_angle
        cl_initial_slope_full = [x for x in cl_initial_slope[1::-1]] + cl_initial_slope
        stall_angles_full = [x for x in stall_angles[1::-1]] + stall_angles
        
        # Create foil models from the input arrays
        foils_data = []
        for foil_index in range(len(ca_values_full)):
            if cl_zero_angle_full[foil_index] > 0:
                foil = Foil(
                    cl_zero_angle = cl_zero_angle_full[foil_index],
                    cl_initial_slope = cl_initial_slope_full[foil_index],
                    cd_min = 0.01,
                    mean_positive_stall_angle = stall_angles_full[foil_index],
                    mean_negative_stall_angle = 2.0 * stall_angles_full[foil_index]
                )
            else:
                foil = Foil(
                    cl_zero_angle = cl_zero_angle_full[foil_index],
                    cl_initial_slope = cl_initial_slope_full[foil_index],
                    cd_min = 0.01,
                    mean_positive_stall_angle = 2.0 * stall_angles_full[foil_index],
                    mean_negative_stall_angle = stall_angles_full[foil_index]
                )
                
            foils_data.append(foil)
                        
        return cls(
            model = VaryingFoil(
                internal_state_data = ca_values_full,
                foils_data = foils_data
            )   
        )
        
    
    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data):
        # Handle the "EffectiveWindSensor" string case (unit variant)
        if data == "EffectiveWindSensor":
            return {'model': EffectiveWindSensor()}
        
        if not isinstance(data, dict):
            return data
        
        # Already in Python/Pydantic form
        if 'model' in data:
            return data
        
        # Rust externally-tagged enum format
        if 'Foil' in data:
            return {'model': Foil(**data['Foil'])}
        elif 'VaryingFoil' in data:
            return {'model': VaryingFoil(**data['VaryingFoil'])}
        elif 'RotatingCylinder' in data:
            return {'model': RotatingCylinder(**data['RotatingCylinder'])}
        else:
            raise ValueError(f"Unknown section model variant: {list(data.keys())}")

    @model_serializer
    def ser_model(self):
        model_dict = self.model.model_dump(exclude_none=True, mode='json')

        if isinstance(self.model, Foil):
            return {
                "Foil": model_dict
            }
        elif isinstance(self.model, VaryingFoil):
            return {
                "VaryingFoil": model_dict
            }
        elif isinstance(self.model, RotatingCylinder):
            return {
                "RotatingCylinder": model_dict
            }
        elif isinstance(self.model, EffectiveWindSensor):
            return "EffectiveWindSensor"
        else:
            raise ValueError("Unsupported section model:", type(self.model))
