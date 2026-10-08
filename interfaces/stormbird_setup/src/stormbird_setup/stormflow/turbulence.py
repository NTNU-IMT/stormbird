"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from typing import Any
from enum import Enum

from pydantic import Field, model_serializer, model_validator

from ..base_model import StormbirdSetupBaseModel


class RealizableKEpsilon(StormbirdSetupBaseModel):
    """
    Coefficients of the realizable k-epsilon model, with the same default values as in OpenFOAM's
    `realizableKE`.
    """

    a0: float = 4.0
    c2: float = 1.9
    sigma_k: float = 1.0
    sigma_epsilon: float = 1.2


# The name of each variant in the Rust `TurbulenceModel` enum, mapped to its Python class
_MODEL_VARIANTS: dict[str, type[StormbirdSetupBaseModel]] = {
    "RealizableKEpsilon": RealizableKEpsilon,
}


class TurbulenceModel(StormbirdSetupBaseModel):
    """
    Wrapper representing the Rust `TurbulenceModel` enum. Serializes into the externally-tagged form
    expected by serde, e.g. `{"RealizableKEpsilon": {...}}`.
    """

    model: RealizableKEpsilon = Field(default_factory=RealizableKEpsilon)

    @classmethod
    def new_realizable_k_epsilon(
        cls, coefficients: RealizableKEpsilon | None = None
    ) -> "TurbulenceModel":
        return cls(model=coefficients if coefficients is not None else RealizableKEpsilon())

    @model_validator(mode="before")
    @classmethod
    def _deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Already in Python/Pydantic form
        if isinstance(data, dict) and "model" in data:
            return data

        # Rust externally-tagged enum form, e.g. {"RealizableKEpsilon": {...}}
        if isinstance(data, dict) and len(data) == 1:
            (tag, inner), = data.items()

            if tag in _MODEL_VARIANTS:
                return {"model": _MODEL_VARIANTS[tag].model_validate(inner)}

        return data

    @model_serializer
    def _serialize_to_rust_enum(self) -> dict[str, Any]:
        for tag, variant in _MODEL_VARIANTS.items():
            if isinstance(self.model, variant):
                return {tag: self.model.model_dump(exclude_none=True, mode="json")}

        raise ValueError(f"Unknown turbulence model variant: {type(self.model)}")


class IntensityAndLengthScale(StormbirdSetupBaseModel):
    """
    `k = 1.5 (I |U|)^2` and `epsilon = C_mu^0.75 k^1.5 / L`, where `I` is the turbulence intensity,
    `|U|` the local inlet velocity, and `L` the turbulent length scale.
    """

    turbulence_intensity: float
    length_scale: float


class IntensityAndViscosityRatio(StormbirdSetupBaseModel):
    """
    `k = 1.5 (I |U|)^2` and `epsilon = C_mu k^2 / (r nu)`, where `r` is the ratio between the eddy
    viscosity and the molecular viscosity, `nu`.
    """

    turbulence_intensity: float = 0.01
    eddy_viscosity_ratio: float = 10.0


class AtmosphericBoundaryLayer(StormbirdSetupBaseModel):
    """
    The equilibrium atmospheric boundary layer of Richards and Hoxey (1993), as used by OpenFOAM's
    `atmBoundaryLayerInlet` conditions: `k = u*^2 / sqrt(C_mu)` and
    `epsilon = u*^3 / (kappa (z + z0))`, where `u*` is the friction velocity, `z` the height above
    the water plane, and `z0` the roughness length.
    """

    friction_velocity: float
    roughness_length: float


# The name of each variant in the Rust `InletTurbulence` enum, mapped to its Python class
_INLET_VARIANTS: dict[str, type[StormbirdSetupBaseModel]] = {
    "IntensityAndLengthScale": IntensityAndLengthScale,
    "IntensityAndViscosityRatio": IntensityAndViscosityRatio,
    "AtmosphericBoundaryLayer": AtmosphericBoundaryLayer,
}


class InletTurbulence(StormbirdSetupBaseModel):
    """
    Wrapper representing the Rust `InletTurbulence` enum, which specifies the turbulence at the
    inlet through k and epsilon. Serializes into the externally-tagged form expected by serde, e.g.
    `{"AtmosphericBoundaryLayer": {...}}`.
    """

    inlet: IntensityAndLengthScale | IntensityAndViscosityRatio | AtmosphericBoundaryLayer = Field(
        default_factory=IntensityAndViscosityRatio
    )

    @classmethod
    def new_intensity_and_length_scale(
        cls, turbulence_intensity: float, length_scale: float
    ) -> "InletTurbulence":
        return cls(
            inlet=IntensityAndLengthScale(
                turbulence_intensity=turbulence_intensity, length_scale=length_scale
            )
        )

    @classmethod
    def new_intensity_and_viscosity_ratio(
        cls, turbulence_intensity: float, eddy_viscosity_ratio: float
    ) -> "InletTurbulence":
        return cls(
            inlet=IntensityAndViscosityRatio(
                turbulence_intensity=turbulence_intensity,
                eddy_viscosity_ratio=eddy_viscosity_ratio,
            )
        )

    @classmethod
    def new_atmospheric_boundary_layer(
        cls, friction_velocity: float, roughness_length: float
    ) -> "InletTurbulence":
        return cls(
            inlet=AtmosphericBoundaryLayer(
                friction_velocity=friction_velocity, roughness_length=roughness_length
            )
        )

    @model_validator(mode="before")
    @classmethod
    def _deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Already in Python/Pydantic form
        if isinstance(data, dict) and "inlet" in data:
            return data

        # Rust externally-tagged enum form, e.g. {"AtmosphericBoundaryLayer": {...}}
        if isinstance(data, dict) and len(data) == 1:
            (tag, inner), = data.items()

            if tag in _INLET_VARIANTS:
                return {"inlet": _INLET_VARIANTS[tag].model_validate(inner)}

        return data

    @model_serializer
    def _serialize_to_rust_enum(self) -> dict[str, Any]:
        for tag, variant in _INLET_VARIANTS.items():
            if isinstance(self.inlet, variant):
                return {tag: self.inlet.model_dump(exclude_none=True, mode="json")}

        raise ValueError(f"Unknown inlet turbulence variant: {type(self.inlet)}")


class WallTreatment(Enum):
    """
    How the turbulence model treats the no-slip walls. The slip walls always use a zero gradient
    condition.

    `WallFunction` (default) uses a zero gradient condition inside the geometries, and standard wall
    functions in the fluid cells close to the wall. `ZeroGradient` only uses the zero gradient
    condition.
    """

    ZeroGradient = "ZeroGradient"
    WallFunction = "WallFunction"


class ConvectionScheme(Enum):
    """
    The discretization of the convection term in the turbulence transport equations.

    `LimitedLinear` (default) is 2nd order and TVD limited, with the same limiter as OpenFOAM's
    `limitedLinear 1`. `Upwind` is 1st order, bounded and very robust, but diffusive.
    """

    Upwind = "Upwind"
    LimitedLinear = "LimitedLinear"


class TurbulenceSolverBuilder(StormbirdSetupBaseModel):
    """
    Settings for the optional RANS turbulence model. The turbulence solver always runs on the same
    platform as the velocity solver. When a turbulence model is used, `effective_viscosity` in the
    `SimulationBuilder` is the molecular viscosity.
    """

    model: TurbulenceModel = Field(default_factory=TurbulenceModel)
    inlet: InletTurbulence = Field(default_factory=InletTurbulence)
    wall_treatment: WallTreatment = WallTreatment.WallFunction
    convection_scheme: ConvectionScheme = ConvectionScheme.LimitedLinear
    nr_jacobi_iterations: int = 2
    """
    The number of Jacobi iterations used to solve the implicit transport equations in each time
    step.
    """
    max_eddy_viscosity: float = 1e5
    """
    Upper limit on the eddy viscosity, as a safeguard against unphysical values, for instance in
    the first time steps.
    """
