'''
Demonstration of the dynamic stall model, using a wing with a sinusoidal variation in the angle of
attack that goes through stall. The oscillation is run for several reduced frequencies, both with
and without the dynamic stall model, and compared against a quasi-steady sweep of the angle of
attack.
'''

import numpy as np
import matplotlib.pyplot as plt

from tqdm import tqdm

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.section_models import SectionModel, Foil, DynamicStallModelBuilder
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, DynamicSettings, QuasiSteadySettings
from stormbird_setup.lifting_line.solver import Iterative
from stormbird_setup.lifting_line.wake import DynamicWakeBuilder, QuasiSteadyWakeSettings, SymmetryCondition
from stormbird_setup.lifting_line.velocity_corrections import VelocityCorrections

from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder

from pystormbird.lifting_line import Simulation

chord = 1.0
span = 8.0
nr_sections = 16
velocity = 10.0
density = 1.225

force_factor = 0.5 * density * chord * span * velocity**2

# The default foil model has a mean stall angle of 20 degrees, so the oscillation goes through stall
mean_angle = np.radians(18.0)
angle_amplitude = np.radians(8.0)

# Reduced frequencies, k = omega * c / (2 V)
reduced_frequencies = [0.025, 0.05, 0.1]
nr_periods = 3

# Non-dimensional time step, in semi-chords travelled, i.e., 2 V dt / c
non_dimensional_time_step = 0.5
time_step = non_dimensional_time_step * chord / (2.0 * velocity)

dynamic_stall = DynamicStallModelBuilder(
    pressure_time_constant = 1.7,
    separation_time_constant = 3.0
)

def build_simulation(*, dynamic: bool, use_dynamic_stall: bool) -> Simulation:
    wing_builder = WingBuilder(
        section_points = [
            SpatialVector(z=0.0),
            SpatialVector(z=span)
        ],
        chord_vectors = [
            SpatialVector(x=chord),
            SpatialVector(x=chord)
        ],
        section_model = SectionModel(model=Foil()),
        dynamic_stall = dynamic_stall if use_dynamic_stall else None
    )

    line_force_model = LineForceModelBuilder(
        nr_sections = nr_sections,
        density = density,
        circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(
            smoothing_length_factor = 0.05
        )
    )

    line_force_model.add_wing_builder(wing_builder)

    solver = Iterative(
        max_iterations_per_time_step = 100,
        damping_factor = 0.05,
        velocity_corrections = VelocityCorrections.new_max_induced_velocity_magnitude()
    )

    if dynamic:
        simulation_settings = DynamicSettings(
            wake = DynamicWakeBuilder(
                symmetry_condition = SymmetryCondition.Z,
                first_panel_relative_length = time_step * velocity / chord
            ),
            solver = solver
        )
    else:
        simulation_settings = QuasiSteadySettings(
            wake = QuasiSteadyWakeSettings(
                symmetry_condition = SymmetryCondition.Z
            ),
            solver = solver
        )

    simulation_builder = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = simulation_settings
    )

    return Simulation(simulation_builder.to_json_string())

def freestream_velocity(simulation: Simulation) -> list[list[float]]:
    return [[velocity, 0.0, 0.0] for _ in simulation.get_freestream_velocity_points()]

def lift_coefficient(simulation: Simulation, angle_of_attack: float, time: float, freestream: list[list[float]]) -> float:
    # A positive angle of attack corresponds to a negative local wing angle in this setup
    simulation.set_local_wing_angles([-angle_of_attack])

    result = simulation.do_step(
        time = time,
        time_step = time_step,
        freestream_velocity = freestream,
    )

    return result.integrated_forces_sum()[1] / force_factor

def quasi_steady_sweep() -> tuple[np.ndarray, np.ndarray]:
    '''
    Static reference, where the angle of attack is increased in small steps over the range of the
    oscillation
    '''
    simulation = build_simulation(dynamic=False, use_dynamic_stall=False)
    freestream = freestream_velocity(simulation)

    angles_of_attack = np.linspace(
        mean_angle - angle_amplitude, mean_angle + angle_amplitude, 50
    )

    cl = [
        lift_coefficient(simulation, angle, float(index), freestream)
        for index, angle in enumerate(angles_of_attack)
    ]

    return angles_of_attack, np.array(cl)

def oscillation(reduced_frequency: float, use_dynamic_stall: bool) -> tuple[np.ndarray, np.ndarray]:
    '''
    Runs the oscillation for the given reduced frequency, and returns the angle of attack and lift
    coefficient for the last period
    '''
    simulation = build_simulation(dynamic=True, use_dynamic_stall=use_dynamic_stall)
    freestream = freestream_velocity(simulation)

    angular_frequency = 2.0 * velocity * reduced_frequency / chord
    period = 2.0 * np.pi / angular_frequency

    nr_time_steps_per_period = int(np.ceil(period / time_step))
    nr_time_steps = nr_periods * nr_time_steps_per_period

    angles_of_attack = []
    cl = []

    for time_index in tqdm(range(nr_time_steps), desc=f"k = {reduced_frequency}, dynamic stall = {use_dynamic_stall}"):
        time = time_index * time_step

        angle_of_attack = mean_angle + angle_amplitude * np.sin(angular_frequency * time)

        angles_of_attack.append(angle_of_attack)
        cl.append(lift_coefficient(simulation, angle_of_attack, time, freestream))

    last_period = slice(nr_time_steps - nr_time_steps_per_period - 1, nr_time_steps)

    return np.array(angles_of_attack)[last_period], np.array(cl)[last_period]

if __name__ == "__main__":
    static_angles, static_cl = quasi_steady_sweep()

    fig, axes = plt.subplots(1, len(reduced_frequencies), sharey=True, figsize=(5 * len(reduced_frequencies), 4.5))

    for ax, reduced_frequency in zip(axes, reduced_frequencies):
        angles_without, cl_without = oscillation(reduced_frequency, use_dynamic_stall=False)
        angles_with, cl_with = oscillation(reduced_frequency, use_dynamic_stall=True)

        ax.plot(np.degrees(static_angles), static_cl, 'k--', label="Quasi-steady")
        ax.plot(np.degrees(angles_without), cl_without, label="Dynamic wake")
        ax.plot(np.degrees(angles_with), cl_with, label="Dynamic wake + dynamic stall")

        ax.set_title(f"k = {reduced_frequency}")
        ax.set_xlabel("Angle of attack [deg]")

    axes[0].set_ylabel("Lift coefficient")
    axes[0].legend()

    plt.tight_layout()
    plt.show()
