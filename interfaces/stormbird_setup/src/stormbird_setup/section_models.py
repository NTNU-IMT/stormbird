"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from enum import Enum
from typing import Any

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

class MultiElementFoil(VaryingFoil):
    '''
    Foil where the internal state typically represents the flap angle (or a number that implicitly 
    sets the flap and leading edge slot angle)
    '''
    pass

class InternalFlowSolverSettings(StormbirdSetupBaseModel):
    '''
    Settings for the numerical solvers used when computing the internal state from the internal
    flow model. Values that are not set use the defaults in Stormbird.
    '''
    segment_tolerance: float | None = None
    segment_max_iterations: int | None = None
    wing_tolerance: float | None = None
    wing_max_iterations: int | None = None

class InternalFlow(StormbirdSetupBaseModel):
    '''
    Model of the flow through the porous surface of a foil with active air flow, e.g., a suction
    sail. The external pressure on the porous surface is modelled as a linear function of the lift
    coefficient, while the flow through the surface is modelled with a loss coefficient. The model
    does not include the fan, which is part of the internal state conversion.
    '''
    cp_zero_cl: float
    cp_gradient: float
    loss_coefficient: float
    porous_fraction: float
    solver_settings: InternalFlowSolverSettings | None = None

class FanCurveData(StormbirdSetupBaseModel):
    '''
    Data points for fan curves. The meaning of the values depends on the type of the fan curves:
    volume flow, pressure rise and power for dimensional curves, and the flow, pressure and power
    coefficients for non-dimensional curves.

    The data must only cover the stable operating range of the fan, where the pressure decreases
    with increasing flow.
    '''
    flow_data: list[float]
    pressure_data: list[float]
    power_data: list[float]

    @model_validator(mode='after')
    def check_data(self) -> "FanCurveData":
        n = len(self.flow_data)

        if n < 2:
            raise ValueError("The fan curves must have at least two data points")

        if len(self.pressure_data) != n or len(self.power_data) != n:
            raise ValueError(
                f"The fan curve data must have the same length. Flow: {n}, "
                f"pressure: {len(self.pressure_data)}, power: {len(self.power_data)}"
            )

        if self.flow_data[0] < 0.0:
            raise ValueError("The flow data for the fan curves can not be negative")

        for i in range(1, n):
            if self.flow_data[i] <= self.flow_data[i - 1]:
                raise ValueError("The flow data for the fan curves must be strictly increasing")

            if self.pressure_data[i] >= self.pressure_data[i - 1]:
                raise ValueError(
                    "The pressure data for the fan curves must be strictly decreasing with "
                    "increasing flow. Only the stable operating range of the fan should be included"
                )

        if any(power < 0.0 for power in self.power_data):
            raise ValueError("The power data for the fan curves can not be negative")

        return self

class FanCurvesType(Enum):
    Dimensional = "Dimensional"
    NonDimensional = "NonDimensional"

class FanCurves(StormbirdSetupBaseModel):
    '''
    Fan curves for a single fan, giving the pressure rise and power as a function of the volume
    flow. The curves are scaled to different rotational speeds and densities using the fan affinity
    laws. The rotational speed is given in revolutions per second. There are two types:

    - Dimensional: the data is given as volume flow [m^3/s], pressure rise [Pa] and power [W],
      measured at a reference rotational speed and density, which must then be set.
    - NonDimensional: the data is given as the flow coefficient, Q / (n D^3), the pressure
      coefficient, dp / (rho n^2 D^2), and the power coefficient, P / (rho n^3 D^5), where D is the
      fan diameter, which must then be set.
    '''
    curves_type: FanCurvesType
    data: FanCurveData
    reference_rotational_speed: float | None = None
    reference_density: float | None = None
    diameter: float | None = None

    @classmethod
    def new_dimensional(
        cls,
        *,
        reference_rotational_speed: float,
        reference_density: float,
        volume_flow_data: list[float],
        pressure_rise_data: list[float],
        power_data: list[float]
    ) -> "FanCurves":
        return cls(
            curves_type = FanCurvesType.Dimensional,
            reference_rotational_speed = reference_rotational_speed,
            reference_density = reference_density,
            data = FanCurveData(
                flow_data = volume_flow_data,
                pressure_data = pressure_rise_data,
                power_data = power_data
            )
        )

    @classmethod
    def new_non_dimensional(
        cls,
        *,
        diameter: float,
        flow_coefficient_data: list[float],
        pressure_coefficient_data: list[float],
        power_coefficient_data: list[float]
    ) -> "FanCurves":
        return cls(
            curves_type = FanCurvesType.NonDimensional,
            diameter = diameter,
            data = FanCurveData(
                flow_data = flow_coefficient_data,
                pressure_data = pressure_coefficient_data,
                power_data = power_coefficient_data
            )
        )

    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data: Any) -> Any:
        if not isinstance(data, dict):
            return data

        # Already in Python/Pydantic form
        if 'curves_type' in data:
            return data

        # Rust externally-tagged enum format
        for curves_type in FanCurvesType:
            if curves_type.value in data:
                return {'curves_type': curves_type, **data[curves_type.value]}

        raise ValueError(f"Unknown fan curves variant: {list(data.keys())}")

    @model_validator(mode='after')
    def check_parameters(self) -> "FanCurves":
        is_dimensional = self.curves_type == FanCurvesType.Dimensional

        dimensional_parameters_set = self.reference_rotational_speed is not None and \
            self.reference_density is not None

        if is_dimensional and not dimensional_parameters_set:
            raise ValueError(
                "reference_rotational_speed and reference_density must be set for dimensional "
                "fan curves"
            )

        if is_dimensional and self.diameter is not None:
            raise ValueError("diameter is only used for non-dimensional fan curves")

        if not is_dimensional and self.diameter is None:
            raise ValueError("diameter must be set for non-dimensional fan curves")

        if not is_dimensional and (
            self.reference_rotational_speed is not None or self.reference_density is not None
        ):
            raise ValueError(
                "reference_rotational_speed and reference_density are only used for dimensional "
                "fan curves"
            )

        return self

    @model_serializer
    def ser_model(self) -> dict[str, object]:
        data = self.data.model_dump(mode='json')

        match self.curves_type:
            case FanCurvesType.Dimensional:
                return {
                    "Dimensional": {
                        "reference_rotational_speed": self.reference_rotational_speed,
                        "reference_density": self.reference_density,
                        "data": data
                    }
                }
            case FanCurvesType.NonDimensional:
                return {
                    "NonDimensional": {
                        "diameter": self.diameter,
                        "data": data
                    }
                }
            case _:
                raise ValueError("Unknown fan curves type", self.curves_type)

class Fan(StormbirdSetupBaseModel):
    '''
    Model of the fan system that drives the internal flow in a wing. The system can consist of
    multiple identical fans in parallel, which share the total volume flow equally. The duct loss
    coefficient, k, gives the pressure loss between the internal volume and the fans, as
    dp_duct = k * Q^2, with Q being the total volume flow.
    '''
    curves: FanCurves
    number_of_fans: int | None = None
    duct_loss_coefficient: float | None = None

class InternalStateConversionType(Enum):
    SuctionCoefficientDirect = "SuctionCoefficientDirect"
    PowerToPowerCoefficient = "PowerToPowerCoefficient"
    PowerToInternalFlow = "PowerToInternalFlow"
    FanSpeedToInternalFlow = "FanSpeedToInternalFlow"

class InternalStateConversion(StormbirdSetupBaseModel):
    '''
    Defines how the internal state set on the wing is converted to the internal state used by the
    foil model on each segment:

    - SuctionCoefficientDirect: the wing state is the suction coefficient, used directly.
    - PowerToPowerCoefficient: the wing state is the power, converted to a power coefficient.
    - PowerToInternalFlow: the wing state is the fan input power, converted to a suction
      coefficient using the internal flow model and a constant fan efficiency. The internal flow
      model must then be set, while the fan efficiency is optional (default 1.0).
    - FanSpeedToInternalFlow: the wing state is the rotational speed of the fans, in revolutions
      per second, converted to a suction coefficient using the internal flow model and the fan
      curves. Both the internal flow model and the fan must then be set.

    The foil data must be given as a function of the power coefficient for the
    PowerToPowerCoefficient variant, and as a function of the suction coefficient for the other
    variants.
    '''
    conversion_type: InternalStateConversionType
    internal_flow: InternalFlow | None = None
    fan_efficiency: float | None = None
    fan: Fan | None = None

    @classmethod
    def new_power_to_internal_flow(
        cls,
        internal_flow: InternalFlow,
        fan_efficiency: float | None = None
    ) -> "InternalStateConversion":
        return cls(
            conversion_type = InternalStateConversionType.PowerToInternalFlow,
            internal_flow = internal_flow,
            fan_efficiency = fan_efficiency
        )

    @classmethod
    def new_fan_speed_to_internal_flow(
        cls,
        internal_flow: InternalFlow,
        fan: Fan
    ) -> "InternalStateConversion":
        return cls(
            conversion_type = InternalStateConversionType.FanSpeedToInternalFlow,
            internal_flow = internal_flow,
            fan = fan
        )

    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Unit variants are represented as strings
        if isinstance(data, str):
            return {'conversion_type': InternalStateConversionType(data)}

        if not isinstance(data, dict):
            return data

        # Already in Python/Pydantic form
        if 'conversion_type' in data:
            return data

        # Rust externally-tagged enum format
        if 'PowerToInternalFlow' in data:
            return {
                'conversion_type': InternalStateConversionType.PowerToInternalFlow,
                **data['PowerToInternalFlow']
            }
        elif 'FanSpeedToInternalFlow' in data:
            return {
                'conversion_type': InternalStateConversionType.FanSpeedToInternalFlow,
                **data['FanSpeedToInternalFlow']
            }
        else:
            raise ValueError(f"Unknown internal state conversion variant: {list(data.keys())}")

    @model_validator(mode='after')
    def check_parameters(self) -> "InternalStateConversion":
        uses_internal_flow = self.conversion_type in (
            InternalStateConversionType.PowerToInternalFlow,
            InternalStateConversionType.FanSpeedToInternalFlow
        )
        uses_fan_efficiency = self.conversion_type == InternalStateConversionType.PowerToInternalFlow
        uses_fan = self.conversion_type == InternalStateConversionType.FanSpeedToInternalFlow

        name = self.conversion_type.value

        if uses_internal_flow and self.internal_flow is None:
            raise ValueError(f"internal_flow must be set for the {name} conversion")

        if not uses_internal_flow and self.internal_flow is not None:
            raise ValueError(f"internal_flow is not used by the {name} conversion")

        if uses_fan and self.fan is None:
            raise ValueError(f"fan must be set for the {name} conversion")

        if not uses_fan and self.fan is not None:
            raise ValueError(f"fan is not used by the {name} conversion")

        if not uses_fan_efficiency and self.fan_efficiency is not None:
            raise ValueError(f"fan_efficiency is not used by the {name} conversion")

        return self

    @model_serializer
    def ser_model(self) -> dict[str, object] | str:
        match self.conversion_type:
            case InternalStateConversionType.SuctionCoefficientDirect | \
                InternalStateConversionType.PowerToPowerCoefficient:
                return self.conversion_type.value
            case InternalStateConversionType.PowerToInternalFlow:
                if self.internal_flow is None:
                    raise ValueError("internal_flow must be set for the PowerToInternalFlow conversion")

                variant_data: dict[str, object] = {
                    "internal_flow": self.internal_flow.model_dump(exclude_none=True, mode='json')
                }

                if self.fan_efficiency is not None:
                    variant_data["fan_efficiency"] = self.fan_efficiency

                return {"PowerToInternalFlow": variant_data}
            case InternalStateConversionType.FanSpeedToInternalFlow:
                if self.internal_flow is None or self.fan is None:
                    raise ValueError(
                        "internal_flow and fan must be set for the FanSpeedToInternalFlow conversion"
                    )

                return {
                    "FanSpeedToInternalFlow": {
                        "internal_flow": self.internal_flow.model_dump(exclude_none=True, mode='json'),
                        "fan": self.fan.model_dump(exclude_none=True, mode='json')
                    }
                }
            case _:
                raise ValueError("Unknown internal state conversion", self.conversion_type)

class ActiveAirFlowFoil(StormbirdSetupBaseModel):
    '''
    Foil with active flow control, e.g., suction. The varying foil model gives the lift and drag as
    a function of the internal state, while the internal state conversion defines how the wing
    internal state is converted to the internal state of the varying foil.
    '''
    varying_foil: VaryingFoil
    internal_state_conversion: InternalStateConversion

class RotatingCylinder(StormbirdSetupBaseModel):
    spin_ratio_data: list[float] | None = None
    cl_data: list[float]| None = None
    cd_start: float | None = None
    cd_end: float | None = None
    cd_angle_start: float | None = None
    cd_spin_ratio_low_drag: float | None = None
    added_mass_factor: float | None = None
    moment_of_inertia_2d: float | None = None
    cdi_correction_factor: float | None = None
    power_coefficient_polynomial: list[float] | None = None
    
class EffectiveWindSensor(StormbirdSetupBaseModel):
    pass

class SectionModel(StormbirdSetupBaseModel):
    model: Foil | MultiElementFoil | ActiveAirFlowFoil | RotatingCylinder | EffectiveWindSensor
    
    @classmethod
    def rotor_sail_deybach_2024(cls, linearized: bool = False) -> "SectionModel":
        """
        Model of a rotor sail that is tuned based on the data in: Deybach, 2024, Aerodynamics of the 
        Flettner rotor at high Reynolds number, RINA Wind propulsion conference
        """

        spin_ratio_data = [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0]
        
        if linearized:
            cl_1 = 2.6596

            cl_data = []
            for sigma in spin_ratio_data:
                cl_data.append(sigma * cl_1)
        else:
            cl_data = [0.0, 1.3484, 2.6596, 4.6177, 7.1980, 7.9874, 8.0, 9.0, 9.00]
        
        return cls(
            model = RotatingCylinder(
                spin_ratio_data = spin_ratio_data,
                cl_data = cl_data
            )
        )
        
    @classmethod
    def rotor_sail_ostman_2023(cls) -> "SectionModel":
        """
        Model of a rotor sail that is tuned based on data in: Ostman, 2023, Calculation of Flettner 
        rotor forces using lifting line and CFD methods, NuTTS conference
        """
        return cls(
            model = RotatingCylinder(
                spin_ratio_data = [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0],
                cl_data = [0.0, 1.22, 2.56, 5.93, 8.87, 9.56, 10.22, 12.0, 13.00]
            )
        )
    
    @classmethod
    def default_wing_sail_single_element(cls) -> "SectionModel":
        """
        Default model for a single element wing sail, which also equals the default model for the 
        Foil model
        """
        stall_angle = np.radians(20.0)
        
        return cls(
            model = Foil(
                mean_positive_stall_angle = stall_angle,
                mean_negative_stall_angle = stall_angle
            )
        )
        
    @classmethod
    def default_wing_sail_two_element(cls) -> "SectionModel":
        """
        Default values for a two-element wing sail.
        """
        internal_state_data = np.radians([-30.0, 0.0, 30.0]).tolist()
        
        stall_angle = np.radians(15)

        foils_data = [
            Foil(cl_zero_angle = -2.0, cd_min = 0.01, mean_negative_stall_angle = stall_angle),
            Foil(cl_zero_angle = 0.0,  cd_min = 0.01),
            Foil(cl_zero_angle = 2.0,  cd_min = 0.01, mean_positive_stall_angle = stall_angle)
        ]

        return cls(
            model=MultiElementFoil(
                internal_state_data = internal_state_data,
                foils_data=foils_data
            )
        )
    
    @classmethod
    def default_suction_sail_turbosail(cls, scale_factor: float = 1.0) -> "SectionModel":
        """
        Default values from a suction sail, manually tuned so that they roughly represent the 
        results of the originally suction sail, named TurboSail.
        
        The paper that presents TurboSail curves:
        https://www.jmwe.org/uploads/1/0/6/4/106473271/aa_suction_sails_turbosail_ventifoil_cousteau_report.pdf
        
        The default values can also be scaled in a simplified way, through the input 
        "scale_factor". This is intended to quickly be able to model "modern suction" sails, which
        are generally thought to be a bit better, but where detailed lift drag curves for varying
        power coefficients are generally not available. One example is the results in the link below,
        which indicate that max lift as a function of power coefficient is about 18% higher with 
        Bound4Blue's suction sail in 2024, compared to the original Turbosail. 
        
        Bound4Blue's paper: https://bound4blue.com/aerodynamic-optimization-of-the-esail/

        The foil data is given as a function of the power coefficient, so the internal state
        conversion is set to PowerToPowerCoefficient, meaning that the wing internal state is the
        power.
        """
        
        # The power coefficient values used to represent the internal state
        cp_values = [0.0, 0.1187, 0.2161, 0.3389]
        
        cl_zero_angle = (np.array([0.0, 2.6, 3.4, 3.8]) * scale_factor).tolist()
        cl_initial_slope = [2 * np.pi, 2*np.pi * 1.4, 2*np.pi * 1.4, 2*np.pi * 1.4]
        stall_angles = (np.radians([20.0, 23, 27, 29.0]) * scale_factor).tolist()
        
        # Make the model valid for both positive and negative Cp values
        cp_values_full = [-x for x in cp_values[:0:-1]] + cp_values
        cl_zero_angle_full = [-x for x in cl_zero_angle[:0:-1]] + cl_zero_angle
        cl_initial_slope_full = [x for x in cl_initial_slope[:0:-1]] + cl_initial_slope
        stall_angles_full = [x for x in stall_angles[:0:-1]] + stall_angles
        
        # Create foil models from the input arrays
        foils_data = []
        for foil_index in range(len(cp_values_full)):
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
            model = ActiveAirFlowFoil(
                varying_foil = VaryingFoil(
                    internal_state_data = cp_values_full,
                    foils_data = foils_data
                ),
                internal_state_conversion = InternalStateConversion(
                    conversion_type = InternalStateConversionType.PowerToPowerCoefficient
                )
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
        if 'SingleElementFoil' in data:
            return {'model': Foil(**data['SingleElementFoil'])}
        elif 'MultiElementFoil' in data:
            return {'model': MultiElementFoil(**data['MultiElementFoil'])}
        elif 'ActiveAirFlowFoil' in data:
            return {'model': ActiveAirFlowFoil(**data['ActiveAirFlowFoil'])}
        elif 'RotatingCylinder' in data:
            return {'model': RotatingCylinder(**data['RotatingCylinder'])}
        else:
            raise ValueError(f"Unknown section model variant: {list(data.keys())}")

    @model_serializer
    def ser_model(self):
        model_dict = self.model.model_dump(exclude_none=True, mode='json')

        if isinstance(self.model, Foil):
            return {
                "SingleElementFoil": model_dict
            }
        elif isinstance(self.model, MultiElementFoil):
            return {
                "MultiElementFoil": model_dict
            }
        elif isinstance(self.model, ActiveAirFlowFoil):
            return {
                "ActiveAirFlowFoil": model_dict
            }
        elif isinstance(self.model, RotatingCylinder):
            return {
                "RotatingCylinder": model_dict
            }
        elif isinstance(self.model, EffectiveWindSensor):
            return "EffectiveWindSensor"
        else:
            raise ValueError("Unsupported section model:", type(self.model))
