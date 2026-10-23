'''
Step 1 in the suction sail example: fits a foil model to the two-dimensional data for each suction
coefficient, and combines them to a varying foil model, where the internal state is the suction
coefficient. The result is stored in a json file, which is used in the next steps, and compared to
the data in plots.
'''

from pathlib import Path

import numpy as np
import scipy.optimize as opt
import matplotlib.pyplot as plt

from stormbird_setup.section_models import Foil as FoilSetup, VaryingFoil as VaryingFoilSetup
from pystormbird.section_models import Foil as FoilModel

from suction_sail_data import CQ_VALUES_2D, lift_data, drag_data

VARYING_FOIL_FILE = Path(__file__).parent / "fitted_varying_foil.json"


def angles_for_drag_data(
    aoa_lift: np.ndarray,
    cl_lift: np.ndarray,
    cl_drag: np.ndarray
) -> np.ndarray:
    '''
    The drag data is given as a function of the lift coefficient. This function finds the angle of
    attack for each drag data point, using the lift data. Points before the maximum lift coefficient
    in the polar use the pre-stall part of the lift curve, while points after use the post-stall
    part.
    '''
    index_max_lift = np.argmax(cl_lift)
    index_max_drag_polar = np.argmax(cl_drag)

    # Pre-stall part, where the lift increases with the angle of attack
    aoa_pre_stall = aoa_lift[:index_max_lift + 1]
    cl_pre_stall = np.maximum.accumulate(cl_lift[:index_max_lift + 1])

    # Post-stall part, where the lift decreases with the angle of attack
    aoa_post_stall = aoa_lift[index_max_lift:][::-1]
    cl_post_stall = cl_lift[index_max_lift:][::-1]

    angles = np.zeros(len(cl_drag))

    for i, cl in enumerate(cl_drag):
        if i <= index_max_drag_polar:
            angles[i] = np.interp(cl, cl_pre_stall, aoa_pre_stall)
        else:
            angles[i] = np.interp(cl, cl_post_stall, aoa_post_stall)

    return angles


class SuctionFoilTuner:
    '''
    Fits the parameters of a foil model to lift and drag data for a single suction coefficient.
    The lift is fitted first, and the drag after.
    '''
    def __init__(self, cq: float):
        self.cq = cq

        self.aoa_lift_deg, self.cl_lift = lift_data(cq, "2d")
        self.cl_drag, self.cd_drag = drag_data(cq, "2d")

        self.aoa_drag_deg = angles_for_drag_data(self.aoa_lift_deg, self.cl_lift, self.cl_drag)

        # The lift before stall is linear, without slope reduction. The data has few points after
        # stall, so the lift after stall is set to the same value for all suction coefficients,
        # rather than tuned. The value matches the data for the largest suction coefficient.
        self.model_setup = FoilSetup(cl_max_after_stall = 3.0)

    def set_lift_parameters(self, x: np.ndarray) -> None:
        self.model_setup.cl_zero_angle = x[0]
        self.model_setup.cl_initial_slope = x[1]
        self.model_setup.mean_positive_stall_angle = x[2]
        self.model_setup.mean_negative_stall_angle = x[2]
        self.model_setup.stall_range = x[3]

    def set_drag_parameters(self, x: np.ndarray) -> None:
        self.model_setup.cd_min = x[0]
        self.model_setup.cd_second_order_factor = x[1]
        self.model_setup.cd_max_after_stall = x[2]

    def foil_model(self) -> FoilModel:
        return FoilModel(self.model_setup.to_json_string())

    def lift_residuals(self, x: np.ndarray) -> np.ndarray:
        self.set_lift_parameters(x)

        model = self.foil_model()

        return np.array([
            model.lift_coefficient(aoa) for aoa in np.radians(self.aoa_lift_deg)
        ]) - self.cl_lift

    def drag_residuals(self, x: np.ndarray) -> np.ndarray:
        self.set_drag_parameters(x)

        model = self.foil_model()

        return np.array([
            model.drag_coefficient(aoa) for aoa in np.radians(self.aoa_drag_deg)
        ]) - self.cd_drag

    def tune(self) -> FoilSetup:
        index_max_lift = np.argmax(self.cl_lift)

        # Initial guess from the data
        slope_initial = np.polyfit(
            np.radians(self.aoa_lift_deg[:index_max_lift + 1]), self.cl_lift[:index_max_lift + 1], 1
        )[0]

        lift_x0 = [
            self.cl_lift[0],
            slope_initial,
            np.radians(self.aoa_lift_deg[index_max_lift] + 2.0),
            np.radians(4.0)
        ]

        # The stall range is limited, as the data does not determine it well, and as a too sharp
        # stall can be challenging for the lifting line solvers
        lift_bounds = (
            [0.0, 0.0, np.radians(5.0), np.radians(2.0)],
            [10.0, 30.0, np.radians(45.0), np.radians(6.0)]
        )

        lift_result = opt.least_squares(self.lift_residuals, lift_x0, bounds=lift_bounds)

        self.set_lift_parameters(lift_result.x)

        drag_x0 = [self.cd_drag[0], 1.0, 2.0]
        drag_bounds = ([0.0, 0.0, 0.0], [2.0, 20.0, 10.0])

        drag_result = opt.least_squares(self.drag_residuals, drag_x0, bounds=drag_bounds)

        self.set_drag_parameters(drag_result.x)

        return self.model_setup


def mirrored_foil(foil: FoilSetup) -> FoilSetup:
    '''
    A foil model for the suction on the other side of the foil, i.e., for a negative suction
    coefficient. The lift is mirrored around zero angle of attack.
    '''
    return foil.model_copy(update = {
        "cl_zero_angle": -foil.cl_zero_angle,
        "cl_slope_reduction_factor_positive": foil.cl_slope_reduction_factor_negative,
        "cl_slope_reduction_factor_negative": foil.cl_slope_reduction_factor_positive,
        "cl_slope_reduction_onset_angle_positive": foil.cl_slope_reduction_onset_angle_negative,
        "cl_slope_reduction_onset_angle_negative": foil.cl_slope_reduction_onset_angle_positive,
        "mean_positive_stall_angle": foil.mean_negative_stall_angle,
        "mean_negative_stall_angle": foil.mean_positive_stall_angle,
    })


def varying_foil_with_mirrored_foils(cq_values: list[float], foils: list[FoilSetup]) -> VaryingFoilSetup:
    '''
    Combines foil models for positive suction coefficients, starting at zero, into a varying foil
    model. Mirrored foils are added for negative suction coefficients, so that the sign of the
    internal state can be used to set the suction side.
    '''
    return VaryingFoilSetup(
        internal_state_data = [-cq for cq in cq_values[:0:-1]] + list(cq_values),
        foils_data = [mirrored_foil(foil) for foil in foils[:0:-1]] + list(foils)
    )


def positive_foils(varying_foil: VaryingFoilSetup) -> tuple[list[float], list[FoilSetup]]:
    '''
    The suction coefficients and foil models for the positive part of a varying foil model
    '''
    pairs = [
        (cq, foil) for cq, foil in zip(varying_foil.internal_state_data, varying_foil.foils_data)
        if cq >= 0.0
    ]

    return [cq for cq, _ in pairs], [foil for _, foil in pairs]


def fit_varying_foil() -> tuple[VaryingFoilSetup, list[SuctionFoilTuner]]:
    '''
    Fits a foil model for each suction coefficient, and combines them into a varying foil model.
    '''
    tuners = [SuctionFoilTuner(cq) for cq in CQ_VALUES_2D]

    foils = [tuner.tune() for tuner in tuners]

    return varying_foil_with_mirrored_foils(CQ_VALUES_2D, foils), tuners


if __name__ == "__main__":
    varying_foil, tuners = fit_varying_foil()

    VARYING_FOIL_FILE.write_text(varying_foil.to_json_string())

    print(f"Fitted varying foil model written to {VARYING_FOIL_FILE.name}\n")

    print(f"{'Cq':>6} {'cl_0':>6} {'slope':>6} {'stall':>6} {'range':>6} {'cd_min':>7} {'cd_2':>6}")

    for tuner in tuners:
        foil = tuner.model_setup
        print(
            f"{tuner.cq:6.3f} {foil.cl_zero_angle:6.2f} {foil.cl_initial_slope:6.2f} "
            f"{np.degrees(foil.mean_positive_stall_angle):6.1f} "
            f"{np.degrees(foil.stall_range):6.1f} {foil.cd_min:7.3f} {foil.cd_second_order_factor:6.3f}"
        )

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    aoa_model_deg = np.linspace(0.0, 40.0, 200)

    for tuner in tuners:
        color = ax_cl.plot(
            tuner.aoa_lift_deg, tuner.cl_lift, "o", label=f"Data, Cq = {tuner.cq}"
        )[0].get_color()

        ax_cd.plot(tuner.cl_drag, tuner.cd_drag, "o", color=color)

        model = tuner.foil_model()

        cl_model = [model.lift_coefficient(aoa) for aoa in np.radians(aoa_model_deg)]
        cd_model = [model.drag_coefficient(aoa) for aoa in np.radians(aoa_model_deg)]

        ax_cl.plot(aoa_model_deg, cl_model, "-", color=color)
        ax_cd.plot(cl_model, cd_model, "-", color=color)

    ax_cl.legend(title="Points: data, lines: model")

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.tight_layout()
    plt.show()
