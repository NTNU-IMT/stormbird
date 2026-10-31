"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .base_model import StormbirdSetupBaseModel

from enum import Enum
from typing import Any

from pydantic import model_serializer, model_validator

class Foil(StormbirdSetupBaseModel):
    cl_zero_angle: float | None = None
    cl_initial_slope: float | None = None
    cl_slope_reduction_factor_positive: float | None = None
    cl_slope_reduction_factor_negative: float | None = None
    cl_slope_reduction_onset_angle_positive: float | None = None
    cl_slope_reduction_onset_angle_negative: float | None = None
    cl_slope_reduction_range: float | None = None
    cl_max_after_stall: float | None = None
    cd_min: float | None = None
    cd_second_order_factor: float | None = None
    cd_max_after_stall: float | None = None
    cd_power_after_stall: float | None = None
    mean_positive_stall_angle: float | None = None
    mean_negative_stall_angle: float | None = None
    stall_range: float | None = None

class VaryingFoil(StormbirdSetupBaseModel):
    '''
    Foil where the parameters are interpolated between foils given for different internal states.

    If the internal state data only contains values with one sign, e.g., only positive values, the
    foil is assumed to be symmetric with respect to the sign of the internal state. An internal
    state with the opposite sign then gives the mirrored foil at the absolute value of the internal
    state. If the data contains both positive and negative values, the data is used directly.
    '''
    internal_state_data: list[float]
    foils_data: list[Foil]

    @model_validator(mode='after')
    def check_data(self) -> "VaryingFoil":
        n = len(self.internal_state_data)

        if n == 0:
            raise ValueError("The varying foil must have at least one foil in the data")

        if len(self.foils_data) != n:
            raise ValueError(
                f"The internal state data and the foils data must have the same length. Internal "
                f"state data: {n}, foils data: {len(self.foils_data)}"
            )

        for i in range(1, n):
            if self.internal_state_data[i] <= self.internal_state_data[i - 1]:
                raise ValueError(
                    "The internal state data for the varying foil must be strictly increasing"
                )

        return self

class MultiElementFoil(VaryingFoil):
    '''
    Foil where the internal state typically represents the flap angle (or a number that implicitly 
    sets the flap and leading edge slot angle)
    '''
    pass

class InternalFlowSolverSettings(StormbirdSetupBaseModel):
    '''
    Settings for the numerical solver used when computing the internal suction of the wing from the
    internal state. Values that are not set use the defaults in Stormbird.
    '''
    wing_tolerance: float | None = None
    wing_max_iterations: int | None = None

class InternalFlow(StormbirdSetupBaseModel):
    '''
    Model of the flow through the porous surface of a foil with active air flow, e.g., a suction
    sail. The external pressure on the porous surface is modelled as a linear function of the
    suction coefficient, C_p,ext = cp_no_suction + cp_suction_gradient * C_Q, while the flow
    through the surface is modelled with a loss coefficient. The model does not include the fan,
    which is part of the internal state conversion.

    The gradient must be zero or negative, i.e., more suction can not increase the external
    pressure, and the loss coefficient and the porous fraction must be larger than zero.
    '''
    cp_no_suction: float
    cp_suction_gradient: float
    loss_coefficient: float
    porous_fraction: float
    solver_settings: InternalFlowSolverSettings | None = None

    @model_validator(mode='after')
    def check_parameters(self) -> "InternalFlow":
        if self.cp_suction_gradient > 0.0:
            raise ValueError(
                "The cp_suction_gradient in the internal flow model must be zero or negative, as "
                f"more suction can not increase the external pressure. Value: {self.cp_suction_gradient}"
            )

        if self.loss_coefficient <= 0.0:
            raise ValueError(
                "The loss_coefficient in the internal flow model must be larger than zero. "
                f"Value: {self.loss_coefficient}"
            )

        if self.porous_fraction <= 0.0:
            raise ValueError(
                "The porous_fraction in the internal flow model must be larger than zero. "
                f"Value: {self.porous_fraction}"
            )

        return self

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
    '''
    Parametric model of a rotating cylinder, e.g., a rotor sail. The lift has an initial slope with
    respect to the spin ratio, which changes smoothly to a different slope at high spin ratios. The
    transition is half-way developed at the transition spin ratio, and takes place over a range
    around it. The lift is an odd function of the spin ratio. Values that are not set use the
    defaults in Stormbird.
    '''
    cl_initial_slope: float | None = None
    cl_high_spin_ratio_slope: float | None = None
    cl_slope_transition_spin_ratio: float | None = None
    cl_slope_transition_range: float | None = None
    cd_start: float | None = None
    cd_end: float | None = None
    cd_angle_start: float | None = None
    cd_spin_ratio_low_drag: float | None = None
    power_coefficient_polynomial: list[float] | None = None
    
class EffectiveWindSensor(StormbirdSetupBaseModel):
    pass

class DynamicStallModelBuilder(StormbirdSetupBaseModel):
    '''
    Dynamic stall model for a wing. The model is set for each wing in the WingBuilder, and can be
    used in dynamic lifting line simulations and in actuator line simulations.

    For section models that build on the foil model, i.e., Foil, MultiElementFoil and
    ActiveAirFlowFoil, the model applies two first order time lags to the stall in the foil model:
    one to the angle of attack, representing the lag in the pressure distribution, and one to the
    amount of stall, representing the lag in the boundary layer separation. The dynamic amount of
    stall is used for both the lift and the drag. In addition, it adds the lift from a leading edge
    vortex, which is created when the lagged angle of attack exceeds the stall angle, decays with
    the vortex time constant, and is only fed until it has passed the trailing edge, after the
    vortex passage time.

    Experimental: the model can also apply a first order time lag to the internal state of the
    section model, with the internal state time constant. This is mostly intended for rotor sails,
    where the internal state is the spin ratio, and is the only parameter that can be set for
    section models that do not build on the foil model. The lag is off by default.

    The time constants are non-dimensional, given in semi-chords travelled, i.e., relative to
    c / (2 V). A value of zero turns the corresponding lag, or the vortex, off. Values that are not
    set use the defaults in Stormbird, which are 1.7 for the pressure time constant, 3.0 for the
    separation time constant, 6.0 for the vortex time constant, 7.0 for the vortex passage time and
    0.0 for the internal state time constant.
    '''
    pressure_time_constant: float | None = None
    separation_time_constant: float | None = None
    vortex_time_constant: float | None = None
    vortex_passage_time: float | None = None
    internal_state_time_constant: float | None = None

    @model_validator(mode='after')
    def check_time_constants(self) -> "DynamicStallModelBuilder":
        for name in (
            "pressure_time_constant",
            "separation_time_constant",
            "vortex_time_constant",
            "vortex_passage_time",
            "internal_state_time_constant"
        ):
            value = getattr(self, name)

            if value is not None and value < 0.0:
                raise ValueError(f"The {name} can not be negative. Value: {value}")

        return self

    def has_foil_parameters(self) -> bool:
        '''
        Returns true if any of the parameters for the stall in the foil model are set
        '''
        return any(
            value is not None for value in (
                self.pressure_time_constant,
                self.separation_time_constant,
                self.vortex_time_constant,
                self.vortex_passage_time
            )
        )

class SectionModel(StormbirdSetupBaseModel):
    model: Foil | MultiElementFoil | ActiveAirFlowFoil | RotatingCylinder | EffectiveWindSensor
    
    def uses_internal_compartments(self) -> bool:
        '''
        Returns true if the section model has an internal flow model, and can therefore be used
        with several internal compartments
        '''
        if not isinstance(self.model, ActiveAirFlowFoil):
            return False

        return self.model.internal_state_conversion.conversion_type in (
            InternalStateConversionType.PowerToInternalFlow,
            InternalStateConversionType.FanSpeedToInternalFlow
        )

    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data: Any) -> Any:
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
    def ser_model(self) -> dict[str, Any] | str:
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
