import numpy as np
import scipy.interpolate as interpolate

def theodorsen_lift_reduction_data(reduced_frequency: np.ndarray) -> np.ndarray:
    '''
    Reduction of the lift due to dynamic effects according to Theodorsen's function, as presented
    in: "Bøckmann, E., 2015, "Wave Propulsion of Ships", page 28, Figure 3.3
    '''

    x_data = np.array([
        0.0000000000000000, 0.10160218835482548, 0.21101992966002325, 0.3243454474404064, 0.45720984759671746, 0.6213364595545132,
        0.8401719421649079, 1.0668229777256741, 1.3325517780382963, 1.6060961313012891, 2.0007815552950357
    ])

    y_data = np.array([
        0.9999999999999999, 0.8254545454545454, 0.7185454545454544, 0.6552727272727272, 0.6094545454545454, 0.5745454545454544,
        0.5516363636363635, 0.5363636363636362, 0.5254545454545453, 0.5199999999999998, 0.5134545454545453
    ])

    spl = interpolate.splrep(x_data, y_data)

    return interpolate.splev(reduced_frequency, spl)
