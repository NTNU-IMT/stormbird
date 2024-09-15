
import os
from pathlib import Path

class FolderPaths():
    def __init__(self, angle_of_attack_deg):
        self.base_folder = Path("base_folder")
        self.foam_run = Path(os.environ['FOAM_RUN'])
        self.run_folder = self.foam_run / 'stormbird_rectangular_wing_example' / f'aoa_{angle_of_attack_deg}'

