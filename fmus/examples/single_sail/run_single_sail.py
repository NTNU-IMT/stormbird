'''
Example of how to run the Spring.fmu using fmpy
'''

from fmpy import read_model_description, extract
from fmpy.fmi2 import FMU2Slave

import shutil
import matplotlib.pyplot as plt

if __name__ == '__main__':
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
        instanceName = 'spring1'
    )

    start_time = 0.0
    stop_time = 20.0
    step_size = 0.1

     # initialize
    fmu.instantiate()
    fmu.setupExperiment(startTime=start_time)
    fmu.enterInitializationMode()

    fmu.setString([vrs['parameters_path']], ["stormbird_parameters.json"])
    fmu.setReal([vrs['wind_velocity']], [10.0])

    fmu.exitInitializationMode()

    # simulation loop
    time = start_time
    while time < stop_time:
        # perform one step
        fmu.doStep(
            currentCommunicationPoint = time, 
            communicationStepSize = step_size
        )

        # advance the time
        time += step_size


    fmu.terminate()
    fmu.freeInstance()

    # clean up
    shutil.rmtree(unzipdir, ignore_errors=True)


