
from typing import Any

from pydantic import field_serializer, field_validator, SerializerFunctionWrapHandler

from ..base_model import StormbirdSetupBaseModel
from .velocity_variation import LogarithmicModel, PowerModel
from .gust_spectrums import DiscretizedSpectrum

# Names of the variants in the Rust `VelocityVariation` enum. Serde serializes this enum in its
# default, externally-tagged form (e.g. `{"Constant": 5.0}` or `{"PowerModel": {...}}`), so the JSON
# produced here must match those tags.
_CONSTANT_TAG = "Constant"
_POWER_MODEL_TAG = "PowerModel"
_LOGARITHMIC_MODEL_TAG = "LogarithmicModel"

_VELOCITY_VARIATION_TAGS = (_CONSTANT_TAG, _POWER_MODEL_TAG, _LOGARITHMIC_MODEL_TAG)


class WindCondition(StormbirdSetupBaseModel):
    direction_coming_from: float
    velocity_variation: LogarithmicModel | PowerModel | float
    parallel_gust: DiscretizedSpectrum | None = None
    perpendicular_gust: DiscretizedSpectrum | None = None
    vertical_gust: DiscretizedSpectrum | None = None

    @field_validator("velocity_variation", mode="before")
    @classmethod
    def _unwrap_velocity_variation(cls, value: Any) -> Any:
        """
        Accepts the externally-tagged representation used by the Rust `VelocityVariation` enum
        (e.g. `{"Constant": 5.0}`) and unwraps it into the inner value so that the union field can
        validate it. Non-tagged input is passed through unchanged.
        """
        if isinstance(value, dict) and len(value) == 1:
            (tag, inner), = value.items()

            if tag in _VELOCITY_VARIATION_TAGS:
                return inner

        return value

    @field_serializer("velocity_variation", mode="wrap")
    def _serialize_velocity_variation(
        self,
        value: LogarithmicModel | PowerModel | float,
        handler: SerializerFunctionWrapHandler,
    ) -> dict[str, Any]:
        """
        Serializes `velocity_variation` into the externally-tagged form expected by the Rust
        `VelocityVariation` enum.
        """
        serialized = handler(value)

        if isinstance(value, LogarithmicModel):
            return {_LOGARITHMIC_MODEL_TAG: serialized}
        elif isinstance(value, PowerModel):
            return {_POWER_MODEL_TAG: serialized}
        else:
            return {_CONSTANT_TAG: serialized}
