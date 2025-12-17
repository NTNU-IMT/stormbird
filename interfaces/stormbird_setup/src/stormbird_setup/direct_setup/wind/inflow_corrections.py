
from ...base_model import StormbirdSetupBaseModel
from ...direct_setup.spatial_vector import SpatialVector
from ..range import Range

from typing import Any

import math

class InflowCorrectionsSingleDirection(StormbirdSetupBaseModel):
    height_values: list[float]
    magnitude_corrections: list[float]
    angle_corrections: list[float]
    wing_indices: list[Range]
    
    @classmethod
    def from_simulation_results_aligne_with_freestream(
        cls, 
        stormbird_result: dict[str, Any],
        freestream_velocity: SpatialVector,
        up_direction: SpatialVector = SpatialVector(z=1.0)
    ) -> "InflowCorrectionsSingleDirection":
        """
        Helper function used to extract inflow correction factors from a simulation results where
        the wings are set to be aligned with the freestream flow, and where an effective wind sensor
        are used to measure the flow
        """
        
        ctrl_points      = stormbird_result["ctrl_points"]
        angles_of_attack = stormbird_result["force_input"]["angles_of_attack"]
        velocity_raw     = stormbird_result["solver_input_ctrl_points_velocity"]
        wing_indices_raw = stormbird_result["wing_indices"]
        
        height_values = []
        magnitude_corrections = []
        angle_corrections = []
        
        wing_indices = []
        
        for indices in wing_indices_raw:
            wing_indices.append(
                Range(
                    start = indices["start"],
                    end = indices["end"]
                )
            )
            
        for i in range(len(ctrl_points)):
            height = (
                ctrl_points[i]["x"] * up_direction.x + 
                ctrl_points[i]["y"] * up_direction.y + 
                ctrl_points[i]["z"] * up_direction.z
            )
            
            height_values.append(
                height
            )
            
            velocity_magnitude = math.sqrt(
                velocity_raw[i]["x"]**2 + 
                velocity_raw[i]["y"]**2 + 
                velocity_raw[i]["z"]**2
            )
            
            magnitude_corrections.append(velocity_magnitude / freestream_velocity.length())
            
            angle_corrections.append(angles_of_attack[i])
        
        return InflowCorrectionsSingleDirection(
            height_values = height_values,
            magnitude_corrections = magnitude_corrections,
            angle_corrections = angle_corrections,
            wing_indices = wing_indices
        )
    
class InflowCorrections(StormbirdSetupBaseModel):
    apparent_wind_directions: list[float]
    individual_corrections: list[InflowCorrectionsSingleDirection]
