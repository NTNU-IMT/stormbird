import os
import shutil
import argparse
from pathlib import Path

import matplotlib.pyplot as plt

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.openfoam import (
    SimulationBuilder, GridBuilder, RefinementBuilder, WindCondition, DynamicTimeControl
)

from stormbird_settings import StormbirdSettings


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("--angle-of-attack", type=float, required=True, help="Angle of attacks to simulate")
    parser.add_argument("--use-ll-correction", action='store_true')

    args = parser.parse_args()

    angle = args.angle_of_attack

    stormbird_settings = StormbirdSettings(
        angle_of_attack_deg=angle,
        use_ll_correction=args.use_ll_correction
    )

    span = stormbird_settings.span

    simulation_builder = SimulationBuilder(
        grid=GridBuilder(
            start_point=SpatialVector(x=-2.0 * span, y=-2.0 * span, z=0.0),
            end_point=SpatialVector(x=4.0 * span, y=2.0 * span, z=2.0 * span),
            background_mesh_cells_per_representative_length=2,
            actuator_line_refinement=RefinementBuilder(cells_per_representative_length=16, padding=5.0),
        ),
        wind_condition=WindCondition(
            direction_coming_from=0.0,
            velocity_variation=stormbird_settings.velocity
        ),
        linear_velocity=SpatialVector(),
        actuator_line=stormbird_settings.get_actuator_line_setup_builder(),
        time_control=DynamicTimeControl(time_step=0.1, end_time=20.0, max_courant_number=2.0),
    )

    run_folder = Path(os.environ['FOAM_RUN']) / 'stormbird_rectangular_wing_example' / f'aoa_{angle}'

    if run_folder.exists():
        shutil.rmtree(run_folder)

    simulation_builder.write_foam_case(run_folder)
    simulation_builder.run_foam_case(run_folder)

    results = simulation_builder.read_results(run_folder)

    force_x = [force / stormbird_settings.force_factor for force in results.forces['force_0.x']]
    force_y = [force / stormbird_settings.force_factor for force in results.forces['force_0.y']]

    plt.plot(results.forces['time'], force_x, label="x")
    plt.plot(results.forces['time'], force_y, label="y")

    print('Last force, x', force_x[-1])
    print('Last force, y', force_y[-1])

    plt.legend()

    plt.savefig(run_folder / "force_plot.png", dpi=300, bbox_inches='tight')
