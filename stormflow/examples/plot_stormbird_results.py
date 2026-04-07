import numpy as np
import json

import matplotlib.pyplot as plt

if __name__ == "__main__":
    file_path = "stormbird_full_results/full_results_1600.json"
    
    with open(file_path, "r") as f:
        data = json.load(f)
        
    area = 40.0 * 5.0
    velocity = 8.0
    density = 1.0
        
    ctrl_points = data["ctrl_points"]
    
    force_input = data["force_input"]
    
    sectional_forces = data["sectional_forces"]
    
    integrated_forces = data["integrated_forces"][0]["total"]
    
    lift_coefficient = integrated_forces["y"] / (0.5 * area * density * velocity**2)
    
    print(lift_coefficient)
    
    nr_ctrl_points = len(ctrl_points)
    
    ctrl_points_z = np.zeros(nr_ctrl_points)
    circulation_strength = np.zeros(nr_ctrl_points)
    
    
    
    for i in range(len(ctrl_points)):
        ctrl_points_z[i] = float(ctrl_points[i]["z"])
        circulation_strength[i] = force_input["circulation_strength"][i]
    
    plt.plot(ctrl_points_z, -circulation_strength)
    
    plt.show()
