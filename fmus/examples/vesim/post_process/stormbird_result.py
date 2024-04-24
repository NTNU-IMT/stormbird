import json

import numpy as np

import matplotlib.pyplot as plt

if __name__ == "__main__":
    sb_result = json.load(open("../output/stormbird_results.json"))

    n_ctrl_points = len(sb_result["ctrl_points"])

    ctrl_point_z = np.zeros(n_ctrl_points)
    angle_of_attack = np.zeros(n_ctrl_points)

    for i in range(n_ctrl_points):
        ctrl_point_z[i] = sb_result["ctrl_points"][i]["z"]
        angle_of_attack[i] = sb_result["force_input"]["angles_of_attack"][i]


    plt.plot(ctrl_point_z, np.degrees(angle_of_attack))

    plt.show()