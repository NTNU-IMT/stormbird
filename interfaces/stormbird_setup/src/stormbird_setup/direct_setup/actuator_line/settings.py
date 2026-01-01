'''
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
'''

from ...base_model import StormbirdSetupBaseModel

class Gaussian(StormbirdSetupBaseModel):
    chord_factor: float = 0.25
    thickness_factor: float = 0.25

class ProjectionSettings(StormbirdSetupBaseModel):
    projection_function: Gaussian = Gaussian()
    project_normal_to_velocity: bool = False
    project_viscous_lift: bool = False
    project_sectional_drag: bool = False

class SamplingSettings(StormbirdSetupBaseModel):
    use_point_sampling: bool = False
    span_projection_factor: float = 0.5
    neglect_span_projection: bool = False
    extrapolate_end_velocities: bool = False
    remove_span_velocity: bool = False
    correction_factor: float = 1.0

class SolverSettings(StormbirdSetupBaseModel):
    damping_factor: float = 0.1
