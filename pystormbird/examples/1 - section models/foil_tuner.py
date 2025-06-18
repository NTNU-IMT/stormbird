import numpy as np
import scipy.optimize as opt

from pystormbird.section_models import Foil

class FoilTuner:
    '''
    Class used to tune the parameters of a foil model to match a set of experimental data (physical 
    or CFD).
    '''
    def __init__(self, *, angles_of_attack_data, cd_data, cl_data):
        self.angles_of_attack_data = angles_of_attack_data
        self.cd_data = cd_data
        self.cl_data = cl_data

        assert len(angles_of_attack_data) == len(cd_data), "All input data arrays must have the same length."
        assert len(angles_of_attack_data) == len(cl_data), "All input data arrays must have the same length."

        self.number_of_data_points = len(angles_of_attack_data)
        
        self.model = Foil()

    def set_cl_parameters(self, x):
        self.model.cl_initial_slope = x[0]
        self.model.cl_max_after_stall = x[1]
        self.model.mean_positive_stall_angle = x[2]
        self.model.stall_range = x[3]

    def set_cd_parameters(self, x):
        self.model.cd_min = x[0]
        self.model.cd_second_order_factor = x[1]
        self.model.cd_power_after_stall = x[2]
        self.model.cd_max_after_stall = x[3]

    def cl_objective_function(self, x):
        self.set_cl_parameters(x)

        cl_model = np.zeros(self.number_of_data_points)

        for i in range(self.number_of_data_points):
            cl_model[i] = self.model.lift_coefficient(self.angles_of_attack_data[i])

        return np.sum((cl_model - self.cl_data) ** 2)

    def cd_objective_function(self, x):
        self.set_cd_parameters(x)

        cd_model = np.zeros(self.number_of_data_points)

        for i in range(self.number_of_data_points):
            cd_model[i] = self.model.drag_coefficient(self.angles_of_attack_data[i])

        return np.sum((cd_model - self.cd_data) ** 2)
    
    def get_tuned_model(self):
        cl_x = opt.minimize(
            self.cl_objective_function, 
            [2 * np.pi, 0.01910, np.radians(14.0), np.radians(6.0)]
        ).x

        self.set_cl_parameters(cl_x)

        cd_x = opt.minimize(
            self.cd_objective_function, 
            [0.01910, 1.0, 1.4, 1.4],
        ).x

        self.set_cd_parameters(cd_x)

        return self.model


