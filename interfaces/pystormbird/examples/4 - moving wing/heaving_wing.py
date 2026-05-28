'''
Script that simulates a heaving wing with both dynamic and quasi-static lifting line models. The
result are compared against each other and against a theoretical (simplified) model.
'''




import numpy as np

from enum import Enum

import plotly.graph_objects as go
from plotly.subplots import make_subplots
import plotly

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.section_models import SectionModel, Foil
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, DynamicSettings, QuasiSteadySettings
from stormbird_setup.lifting_line.wake import DynamicWakeBuilder

from pystormbird.lifting_line import Simulation

def get_motion_functions(*, amplitude: float, radial_frequency: float):
    '''
    Create closures for the motion as a function of time, based on the amplitude and radial frequency.
    '''
    def position(t: np.ndarray) -> np.ndarray:
        return amplitude * np.sin(radial_frequency * t)

    def velocity(t: np.ndarray) -> np.ndarray:
        return amplitude * radial_frequency * np.cos(radial_frequency * t)

    return position, velocity
    
class SimMode(Enum):
    Dynamic = 0
    QuasiStatic = 1
    
def simulate_heaving_wing(
    *,
    reduced_frequency: float,
    amplitude: float,
    velocity: float,
    chord_length: float,
    span: float,
    sim_mode: SimMode,
    density: float = 1.225,
    nr_sections: int = 32
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """
    Function that simulates a harmonically heaving wing, where the frequency is set based on the 
    supplied reduced frequency
    """
    
    radial_frequency = reduced_frequency * velocity / (0.5 * chord_length)
    frequency = radial_frequency / (2.0 * np.pi)
    period = 1.0 / frequency

    position_func, _ = get_motion_functions(
        amplitude = amplitude, radial_frequency = radial_frequency
    )

    force_factor = 0.5 * chord_length * span * density * velocity**2

    dt = 0.1 * chord_length / velocity
    final_time = 5.0 * period

    wing_builder = WingBuilder(
        section_points = [
            SpatialVector(z=-span/2.0),
            SpatialVector(z=span/2.0)
        ],
        chord_vectors = [
            SpatialVector(x=chord_length),
            SpatialVector(x=chord_length)
        ],
        section_model = SectionModel(model=Foil()),
    )

    line_force_model = LineForceModelBuilder(nr_sections=nr_sections)
    line_force_model.add_wing_builder(wing_builder)
    
    match sim_mode:
        case SimMode.Dynamic:
            dynamic_wake = DynamicWakeBuilder.new_default(
                time_step = dt, 
                chord_length = chord_length, 
                velocity = velocity
            )
        
            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = DynamicSettings(
                    wake = dynamic_wake
                )
            )
        case SimMode.QuasiStatic:
            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = QuasiSteadySettings()
            )
        case _:
            raise ValueError("Uknown simualtion mode", sim_mode.name)
    
    simulation = Simulation(simulation_builder.to_json_string())
    
    time = []
    lift = []
    drag = []

    t = 0.0

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    freestream_velocity = []
    for _ in freestream_velocity_points:
        freestream_velocity.append([velocity, 0.0, 0.0])

    while t < final_time:
        simulation.set_translation_with_velocity_using_finite_difference(
            [0.0, position_func(t), 0.0],
            dt
        )

        result = simulation.do_step(
            time = t,
            time_step = dt,
            freestream_velocity = freestream_velocity,
        )

        forces = result.integrated_forces_sum()

        time.append(t)
        lift.append(forces[1] / force_factor)
        drag.append(forces[0] / force_factor)

        t += dt
        
    return np.array(time), np.array(lift), np.array(drag)

if __name__ == "__main__":
    reduced_frequencies = [0.2, 0.4]
    modes = [SimMode.QuasiStatic, SimMode.Dynamic]
    
    amplitude_factor = 0.1

    velocity = 8.0
    chord_length = 1.0
    span = 32.0
    
    amplitude = amplitude_factor * chord_length
    
    colors = plotly.colors.qualitative.Plotly
    
    fig = go.Figure()
    
    for freq_index, reduced_frequency in enumerate(reduced_frequencies):
        for mode_index, mode in enumerate(modes):
            time, cl, cd = simulate_heaving_wing(
                reduced_frequency = reduced_frequency,
                amplitude = amplitude,
                velocity = velocity,
                chord_length = chord_length,
                span = span,
                sim_mode = mode,
            )
            
            if mode == SimMode.QuasiStatic:
                dash = "dash"
                showlegend=False
            else:
                dash = "solid"
                showlegend=True
            
            fig.add_trace(
                go.Scatter(
                    x = time / time[-1],
                    y = cl,
                    line=dict(color=colors[freq_index], dash=dash),
                    name = f"reduced frequency {reduced_frequency}",
                    showlegend=showlegend
                )
            )
            
    fig.add_trace(
        go.Scatter(
            x=[None],
            y=[None],
            line=dict(color="grey", dash="solid"),
            mode="lines",
            name="Dynamic"
        )
    )
    
    fig.add_trace(
        go.Scatter(
            x=[None],
            y=[None],
            line=dict(color="grey", dash="dash"),
            mode="lines",
            name="Quasi-static"
        )
    )
    
    fig.update_xaxes(title="Time / end time")
    fig.update_yaxes(title="Lift coefficient")
    
    fig.update_layout(
        width = 960,
        height = 480,
        margin=dict(t=20),
        legend = dict(
            orientation = "h",
            yanchor = "top",
            y = -0.15,
            xanchor = "center",
            x = 0.5
        )
    )
    
    fig.write_image("heaving_wing.pdf")
    fig.write_image("heaving_wing.svg")
    fig.write_image("heaving_wing.png", scale=2)
            
    fig.show()
