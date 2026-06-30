"""Type stubs for pystormbird.smoothing."""


class GaussianSmoothing:
    def __init__(self, smoothing_length: float) -> None: ...

    def apply_smoothing(self, x: list[float], y: list[float]) -> list[float]: ...

    def apply_smoothing_with_varying_smoothing_weight(
        self, x: list[float], y: list[float], smoothing_weight: list[float]
    ) -> list[float]: ...
