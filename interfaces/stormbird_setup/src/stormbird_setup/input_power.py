from .base_model import StormbirdSetupBaseModel

class InputPower(StormbirdSetupBaseModel):
    section_models_internal_state_data: list[float]
    input_power_per_wing_data: list[float]