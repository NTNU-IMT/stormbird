"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from pydantic import model_validator, Field

from enum import Enum

import numpy as np

class InternalStateSetPointType(Enum):
    '''
    Defines the units of the internal state set point data:

    - SectionModel: the data is given in the units of the section model, e.g., spin ratio for rotor
      sails or the suction/power coefficient for suction sails, and is converted to the wing
      internal state.
    - Wing: the data is given directly as the wing internal state, e.g., the power (with the sign
      setting the direction) for suction sails, and is used without conversion.
    '''
    SectionModel = "SectionModel"
    Wing = "Wing"

class ControllerSetPoints(StormbirdSetupBaseModel):
    '''
    Set points for a sail, as a function of the apparent wind direction.

    The internal state data is given in the units specified by internal_state_set_point_type. The
    max value and max change rate for the internal state are applied to the wing internal state,
    after any conversion from the set point data.
    '''
    apparent_wind_directions_data: list[float]
    angle_of_attack_data: list[float] | None = None
    internal_state_data: list[float] | None = None
    internal_state_set_point_type: InternalStateSetPointType = InternalStateSetPointType.SectionModel
    wing_internal_state_max_value: float | None = None
    use_effective_angle_of_attack: bool = False
    max_local_wing_angle_change_rate: float | None = None
    wing_internal_state_max_change_rate: float | None = None
                
    @classmethod
    def new_default_wing_sail_single_element(cls, max_angle_deg: float = 15.0) -> "ControllerSetPoints":
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180])
        angle_of_attack_data = np.radians([-max_angle_deg, -max_angle_deg, 0.0, 0.0, max_angle_deg, max_angle_deg])

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            angle_of_attack_data = angle_of_attack_data.tolist()
        )
        
    @classmethod
    def new_default_wing_sail_two_element(cls, max_angle_deg: float = 12.0) -> "ControllerSetPoints":
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180])
        angle_of_attack_data = np.radians([-max_angle_deg, -max_angle_deg, 0.0, 0.0, max_angle_deg, max_angle_deg])
        internal_state_data = np.radians([-30.0, -30.0, 0.0, 0.0, 30.0, 30.0])

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            angle_of_attack_data = angle_of_attack_data.tolist(),
            internal_state_data = internal_state_data.tolist()
        )
        
    @classmethod
    def new_default_rotor_sail(cls, *, max_rps: float | None = None) -> "ControllerSetPoints":
        '''
        Helper function to quickly set up a suitable controller for a rotor sail. Assumed to be
        fairly general.
        
        Args:
            max_rps: Maximum revolutions per second. If provided, limits the rotor speed.
                     The controller set points use spin ratio (non-dimensional), and the 
                     conversion to RPS happens internally based on the local velocity and 
                     cylinder diameter.
        '''
        apparent_wind_directions_data = np.radians([-180, -40, -15, 15, 40, 180])
        # Set points are in spin ratio (non-dimensional)
        internal_state_data = [3.0, 3.0, 0.0, 0.0, -3.0, -3.0]

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data.tolist(),
            internal_state_data = internal_state_data,
            wing_internal_state_max_value = max_rps  # Max value is in RPS (wing state)
        )
        
    @classmethod
    def new_default_suction_sail(
        cls, 
        max_aoa_deg: float = 30.0, 
        max_power_coefficient: float = 0.3,
        max_power: float | None = None
    ) -> "ControllerSetPoints":
        '''
        Helper function to quickly set up a suitable controller for a suction sail, where the
        section model uses the PowerToPowerCoefficient internal state conversion, such as the
        default TurboSail model. The set points are then given as power coefficients, and the wing
        internal state is the power.

        Args:
            max_aoa_deg: Maximum angle of attack in degrees.
            max_power_coefficient: Maximum power coefficient set point (non-dimensional).
            max_power: Maximum power in Watts. If provided, limits the actual power
                       regardless of what the power coefficient set point would require.
        '''
        apparent_wind_directions_data = np.radians([-180, -15, -10, 10, 15, 180]).tolist()
        
        angle_of_attack_data = np.radians([
            -max_aoa_deg, -max_aoa_deg, 0.0, 
            0.0, max_aoa_deg, max_aoa_deg
        ]).tolist()
        
        # Set points are in power coefficient (non-dimensional)
        internal_state_data = [
            -max_power_coefficient, -max_power_coefficient, 0.0, 
            0.0, max_power_coefficient, max_power_coefficient
        ]

        return ControllerSetPoints(
            apparent_wind_directions_data = apparent_wind_directions_data,
            angle_of_attack_data = angle_of_attack_data,
            internal_state_data = internal_state_data,
            wing_internal_state_max_value = max_power  # Max value is in Watts (wing state)
        )


class SpanwiseMeasurementBuilder(StormbirdSetupBaseModel):
    '''
    Defines the part of the span of each wing that the controller measures the flow on. The two
    locations are non-dimensional, and run from -0.5 at one end of a wing to 0.5 at the other.

    Stormbird uses the control point that lies closest to each of the two locations, so a small
    deviation from them is expected: the control points are what a simulation resolves. Both of them
    are included in the measurement, so the same start and end location measures the single control
    point that lies closest to it. Each wing gets its own indices, since the wings can be built with
    a different number of sections.
    '''
    non_dim_start_location: float = -0.25
    non_dim_end_location: float = 0.25

    @model_validator(mode='after')
    def check_locations(self) -> "SpanwiseMeasurementBuilder":
        if self.non_dim_end_location < self.non_dim_start_location:
            raise ValueError(
                "The end of a spanwise measurement cannot be before its start. The start location "
                f"is {self.non_dim_start_location} and the end location is "
                f"{self.non_dim_end_location}"
            )

        return self

class ControllerBuilder(StormbirdSetupBaseModel):
    set_points: list[ControllerSetPoints]
    spanwise_measurement: SpanwiseMeasurementBuilder = Field(
        default_factory=lambda: SpanwiseMeasurementBuilder()
    )
    time_steps_between_updates: int = 1
    start_time: float = 0.0
    moving_average_window_size: int | None = None
    use_input_velocity_for_apparent_wind_direction: bool = False
