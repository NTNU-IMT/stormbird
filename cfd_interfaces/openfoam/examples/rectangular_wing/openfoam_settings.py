import os
from pathlib import Path

from dataclasses import dataclass

@dataclass(kw_only=True, slots=True)
class SimulationSettings:
    number_of_threads: int | None = None

    def __post_init__(self):
        if self.number_of_threads is None:
            self.number_of_threads = int(os.cpu_count() / 2)

    def write_to_file(self, path: str) -> None:
        with open(path, 'w') as f:
            for slot in self.__slots__:
                value = getattr(self, slot)

                f.write(f"{slot} {value};\n")

    def set_as_environmental_variables(self):
        for slot in self.__slots__:
            value = getattr(self, slot)

            os.environ[slot] = str(value)

class FolderPaths():
    def __init__(self, angle_of_attack_deg):
        self.base_folder = Path("base_folder")
        self.foam_run = Path(os.environ['FOAM_RUN'])
        self.run_folder = self.foam_run / 'stormbird_rectangular_wing_example' / f'aoa_{angle_of_attack_deg}'