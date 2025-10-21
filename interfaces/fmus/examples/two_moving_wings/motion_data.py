import pandas as pd
import numpy as np

def write_motion_data(
    *,
    period: float,
    end_time: float,
    time_step: float,
    file_path: str
):
    t = np.arange(0, end_time + 1e-6, time_step)

    omega = 2 * np.pi / period

    x_position = np.zeros_like(t)
    y_position = 0.0 * np.sin(omega * t)
    x_rotation = 10.0 * np.sin(omega * t)

    out_dict = {
        "time": t,
        "x_position": x_position,
        "y_position": y_position,
        "z_position": np.zeros_like(t),
        "x_rotation": x_rotation,
        "y_rotation": np.zeros_like(t),
        "z_rotation": np.zeros_like(t),
    }

    df = pd.DataFrame(out_dict)

    df.to_csv(file_path, index=False)
