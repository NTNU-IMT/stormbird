'''
Functions for loading the digitized Turbosail data, shared by the scripts in this example.

The data is digitized from: Charrier et al. (1985), Fondation Cousteau and windship propulsion
1980 - 1985, System Cousteau - Pechiney, Journal of Wind Engineering and Industrial Aerodynamics,
20, 39-60.
'''

from pathlib import Path

import numpy as np
import polars as pl

DATA_FOLDER = Path(__file__).parent / "data"

# Suction coefficients with two-dimensional data (figure 8 in the paper)
CQ_VALUES_2D = [0.0, 0.018, 0.023, 0.026, 0.032, 0.042, 0.047]

# Suction coefficients with three-dimensional data (figure 9 in the paper)
CQ_VALUES_3D = [0.027, 0.036, 0.05]

# The caption of figure 8 in the paper states that "the scale of CD is double that of CL". The
# two-dimensional drag data is digitized using the same scale as the lift, and is therefore
# multiplied with this factor to get the actual drag coefficient. This is also necessary for the
# two-dimensional drag to be lower than the three-dimensional drag, which includes induced drag.
CD_2D_SCALE_FACTOR = 0.5

# The three-dimensional model in the paper had an aspect ratio of 4, with one end against the wall
# of the wind tunnel, which gives an effective aspect ratio of 8. The power curve is assumed to be
# for the same model.
ASPECT_RATIO_3D = 4.0


def read_csv(file_name: str) -> pl.DataFrame:
    '''
    Reads a csv file with digitized data, where the values and column names can contain extra
    white space.
    '''
    return (
        pl.read_csv(DATA_FOLDER / file_name, infer_schema=False)
            .rename(lambda column: column.strip())
            .with_columns(pl.all().str.strip_chars().cast(pl.Float64))
    )


def lift_data(cq: float, dimension: str) -> tuple[np.ndarray, np.ndarray]:
    '''
    Returns the angles of attack, in degrees, and the lift coefficients for a suction coefficient.
    The dimension is either "2d" or "3d".
    '''
    data = read_csv(f"cl_{dimension}_cq_{cq:.3f}.csv")

    return data["aoa [deg]"].to_numpy(), data["cl"].to_numpy()


def drag_data(cq: float, dimension: str) -> tuple[np.ndarray, np.ndarray]:
    '''
    Returns the lift and drag coefficients for a suction coefficient, as given in the polar plots.
    The dimension is either "2d" or "3d". The scale factor is applied to the two-dimensional drag.
    '''
    data = read_csv(f"cd_{dimension}_cq_{cq:.3f}.csv")

    scale_factor = CD_2D_SCALE_FACTOR if dimension == "2d" else 1.0

    return data["cl"].to_numpy(), data["cd"].to_numpy() * scale_factor


def power_curve_data() -> tuple[np.ndarray, np.ndarray]:
    '''
    Returns the power coefficient and the lift coefficient for the three-dimensional model.

    The power coefficient in the paper is defined as Ca = P / (0.5 rho W^3 S), and is computed from
    measurements of the suction coefficient and the pressure in the internal chamber, Kp, as
    Ca = Cq (|Kp| + 1). That is, it is the minimum power needed to suck the air into the chamber and
    discharge it to the outer flow at the speed of the wind, and not a direct measurement of the
    power of a fan.
    '''
    data = read_csv("cp_cl_curve.csv")

    # The first column is the power coefficient, the second is the lift coefficient
    return data[data.columns[0]].to_numpy(), data[data.columns[1]].to_numpy()
