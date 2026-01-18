

class LineForceModel:
    def __ini__(self, json_string: str) -> None: ...
    
    @property
    def wing_indices(self) -> list[list[int]]: ...