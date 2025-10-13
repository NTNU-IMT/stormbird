'''
Example of how to run the Spring.fmu using fmpy
'''

from fmpy import read_model_description, extract
from fmpy.fmi2 import FMU2Slave

import shutil
import numpy as np
import matplotlib.pyplot as plt

import argparse

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Run the Spring.fmu example.')
    parser.add_argument('--quasi-steady', action='store_true', help='Run the quasi-static example.')

    fmu_filename = '../../stormbird_lifting_line/StormbirdLiftingLine.fmu'
    
    model_description = read_model_description(fmu_filename)

    vrs = {}
    for variable in model_description.modelVariables:
        vrs[variable.name] = variable.valueReference
    
    unzipdir = extract(fmu_filename)

    fmu = FMU2Slave(
        guid = model_description.guid,
        unzipDirectory = unzipdir,
        modelIdentifier = model_description.coSimulation.modelIdentifier,
        instanceName = 'stormbird_instance'
    )

    start_time = 0.0
    stop_time = 20.0
    step_size = 0.1

    wind_velocity = 10.0
    wind_direction_coming_from = 0.0
    ship_velocity = 5.0

    local_wing_angle = 10

    surge_velocity = ship_velocity + wind_velocity * np.cos(np.radians(wind_direction_coming_from))
    sway_velocity = wind_velocity * np.sin(np.radians(wind_direction_coming_from))

    total_velocity_squared = surge_velocity**2 + sway_velocity**2

    area = 33*11
    density = 1.225

    force_factor = 0.5 * density * area * total_velocity_squared

    if parser.parse_args().quasi_steady:
        parameters_path = 'stormbird_parameters_quasi_steady.json'
    else:
        parameters_path = 'stormbird_parameters_dynamic.json'

     # initialize
    fmu.instantiate()
    fmu.setupExperiment(startTime=start_time)
    fmu.enterInitializationMode()

    fmu.setString([vrs['parameters_path']], [parameters_path])
    fmu.setReal([vrs['wind_velocity']], [wind_velocity])
    fmu.setReal([vrs['wind_direction_coming_from']], [np.radians(wind_direction_coming_from)])
    fmu.setReal([vrs['local_wing_angle_1']], [local_wing_angle])

    fmu.exitInitializationMode()

    f_x_array = []
    f_y_array = []
    f_z_array = []
    time_array = []

    # simulation loop
    time = start_time
    while time < stop_time:
        translation_x = time * ship_velocity
        fmu.setReal([vrs['translation_x']], [translation_x])

        # perform one step
        fmu.doStep(
            currentCommunicationPoint = time, 
            communicationStepSize = step_size
        )

        f_x, f_y, f_z = fmu.getReal([vrs['force_x'], vrs['force_y'], vrs['force_z']])

        f_x_array.append(f_x / force_factor)
        f_y_array.append(f_y / force_factor)
        f_z_array.append(f_z / force_factor)
        time_array.append(time)

        # advance the time
        time += step_size

    fmu.terminate()
    fmu.freeInstance()

    # clean up
    shutil.rmtree(unzipdir, ignore_errors=True)

    plt.plot(time_array, f_y_array, label='f_y')

    plt.ylim(0, 1.0)

    plt.xlabel('time (s)')
    plt.ylabel('Force (N)')
    plt.title('Force on the sail')
    plt.legend()

    plt.show()


