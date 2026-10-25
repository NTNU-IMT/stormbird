'''
Two rotor sails on the SOBC1 ship, where the superstructure is modelled as a slip geometry. The rotor
speed is set by a controller, based on the apparent wind direction at each rotor.
'''

import os
import shutil
import argparse
from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.section_models import SectionModel, RotatingCylinder
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.input_power import InputPowerModel, InputPowerDataType
from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.controller import ControllerBuilder, ControllerSetPoints
from stormbird_setup.actuator_line import (
    ActuatorLineBuilder, LiftingLineCorrectionBuilder, Gaussian, ProjectionSettings,
    SamplingSettings, SolverSettings, SymmetryCondition
)
from stormbird_setup.openfoam import (
    SimulationBuilder, GridBuilder, RefinementBuilder, GeometryBuilder, WindCondition,
    DynamicTimeControl
)

DIAMETER = 5.0
ROTOR_POSITIONS = [(30.0, 13.0), (-30.0, 13.0)]
ROTOR_BOTTOM = 9.875
ROTOR_HEIGHT = 30.0

def actuator_line_builder() -> ActuatorLineBuilder:
    line_force_model = LineForceModelBuilder(
        nr_sections=32,
        circulation_correction=CirculationCorrectionBuilder.new_gaussian_smoothing(
            smoothing_length_factor=0.05
        ),
    )

    for x, y in ROTOR_POSITIONS:
        line_force_model.add_wing_builder(
            WingBuilder(
                section_points=[
                    SpatialVector(x=x, y=y, z=ROTOR_BOTTOM),
                    SpatialVector(x=x, y=y, z=ROTOR_BOTTOM + ROTOR_HEIGHT)
                ],
                chord_vectors=[SpatialVector(x=DIAMETER), SpatialVector(x=DIAMETER)],
                section_model=SectionModel(
                    model=RotatingCylinder(
                        cl_initial_slope=2.65,
                        cl_high_spin_ratio_slope=0.5,
                        cl_slope_transition_spin_ratio=3.9,
                        cl_slope_transition_range=0.5,
                        cd_start=0.686,
                        cd_end=0.1
                    )
                ),
                input_power_model=InputPowerModel(input_power_type=InputPowerDataType.NoPower),
            )
        )

    return ActuatorLineBuilder(
        line_force_model=line_force_model,
        projection_settings=ProjectionSettings(
            projection_function=Gaussian(chord_factor=0.6, thickness_factor=0.6)
        ),
        solver_settings=SolverSettings(damping_factor=0.05),
        sampling_settings=SamplingSettings(remove_span_velocity=True),
        lifting_line_correction=LiftingLineCorrectionBuilder(symmetry_condition=SymmetryCondition.Z),
        controller=ControllerBuilder(
            set_points=[ControllerSetPoints.new_default_rotor_sail() for _ in ROTOR_POSITIONS]
        ),
        write_iterations_full_result=100,
    )

def simulation_builder(end_time: float) -> SimulationBuilder:
    return SimulationBuilder(
        grid=GridBuilder(
            start_point=SpatialVector(x=-300.0, y=-150.0, z=0.0),
            end_point=SpatialVector(x=300.0, y=150.0, z=150.0),
            background_mesh_cells_per_representative_length=1,
            actuator_line_refinement=RefinementBuilder(cells_per_representative_length=8, padding=3.0),
            geometry_cells_per_representative_length=4,
        ),
        wind_condition=WindCondition(direction_coming_from=np.radians(90.0), velocity_variation=10.0),
        linear_velocity=SpatialVector(x=6.17333),
        actuator_line=actuator_line_builder(),
        slip_geometries=[
            GeometryBuilder.new_triangle_mesh(str(Path(__file__).parent / "sobc1_superstructure.obj"))
        ],
        time_control=DynamicTimeControl(time_step=0.1, end_time=end_time, max_courant_number=1.0),
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--end-time", type=float, default=60.0, help="End time of the simulation")

    args = parser.parse_args()

    builder = simulation_builder(args.end_time)

    run_folder = Path(os.environ['FOAM_RUN']) / 'stormbird_rotor_sails_on_sobc1'

    if run_folder.exists():
        shutil.rmtree(run_folder)

    builder.write_foam_case(run_folder)
    builder.run_foam_case(run_folder)

    results = builder.read_results(run_folder)

    for wing_index in range(len(ROTOR_POSITIONS)):
        plt.plot(results.forces['time'], results.forces[f'force_{wing_index}.x'], label=f"Rotor {wing_index}, x")

    print('Last force on each rotor, x:', [results.forces[f'force_{i}.x'][-1] for i in range(len(ROTOR_POSITIONS))])
    print('Last force on each rotor, y:', [results.forces[f'force_{i}.y'][-1] for i in range(len(ROTOR_POSITIONS))])

    plt.legend()
    plt.xlabel("Time [s]")
    plt.ylabel("Force [N]")

    plt.savefig(run_folder / "force_plot.png", dpi=300, bbox_inches='tight')
