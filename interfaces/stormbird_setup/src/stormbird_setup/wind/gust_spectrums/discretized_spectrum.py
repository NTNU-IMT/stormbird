from ...base_model import StormbirdSetupBaseModel

class DiscretizedSpectrum(StormbirdSetupBaseModel):
    frequencies: list[float]
    amplitudes: list[float]
    phases: list[float]
