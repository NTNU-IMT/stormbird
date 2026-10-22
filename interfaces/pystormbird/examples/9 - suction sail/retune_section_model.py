'''
TODO: This script is currently not part of the example, and does not work with the current foil
model. It uses the higher order term in the pre-stall lift (`cl_high_order_factor_positive`,
`cl_high_order_factor_negative` and `cl_high_order_power`), which has been replaced by a reduction
of the lift slope (`cl_slope_reduction_*`). The corrections must be updated to the slope reduction
before the script can be used again. When this is done, set `USE_RETUNED_SECTION_MODEL` in
suction_sail_model.py to use the retuned model in the other steps.

Optional step in the suction sail example: retunes the section model from the first step, so that
the three-dimensional simulations match the three-dimensional data better.

The three-dimensional data for each suction coefficient is first divided into three categories,
based on the angle of attack with the maximum lift: well below stall, close to stall but before the
maximum lift, and after stall. A linear curve is fitted to the lift well below stall, and this
curve is used when comparing to the simulations, as it is smooth and not affected by noise in the
digitized data.

The following parameters are corrected, while the rest are kept as fitted to the two-dimensional
data:

- The lift at zero angle of attack, `cl_zero_angle`, with an offset, and the lift slope,
  `cl_initial_slope`, with a factor.
- A higher order term in the lift, through `cl_high_order_factor_positive` and
  `cl_high_order_factor_negative`, which makes the lift curve bend over before stall, as in the
  data. The same factor is used for positive and negative angles of attack. The power of the term,
  `cl_high_order_power`, is fixed.
- The stall angle, `mean_positive_stall_angle` and `mean_negative_stall_angle`, with an offset.

The tuning is done in two steps:

1) The stall is moved to a very large angle of attack, so that it does not affect the lift in the
   range of the data. The lift corrections are then tuned to the linear curve well below stall and
   to the data close to stall, at the same time. As the power of the higher order term is fixed,
   the lift from the simulations is close to linear in the corrections. They are therefore found
   with a few Gauss-Newton iterations, where the sensitivity of the lift to each correction is found
   with finite differences.
2) The stall angle offset is tuned for each suction coefficient, so that the maximum lift is the
   same as in the data, with a bracketed root finding method.

To get a smooth variation with the suction coefficient, the corrections are not tuned for each foil
model separately. Instead, each correction is a function of the suction coefficient, defined by its
value at the suction coefficients with three-dimensional data, and linearly interpolated between
them. Outside these suction coefficients, the closest value is used. Each foil model then gets the
corrections at its own suction coefficient, so that foil models with similar suction coefficients
get similar corrections.

The largest suction coefficient in the three-dimensional data is larger than the largest suction
coefficient in the two-dimensional data. A foil model for this suction coefficient is therefore
added, based on the foil model for the largest suction coefficient in the two-dimensional data, so
that it can be tuned directly to the three-dimensional data.
'''

from dataclasses import dataclass

import numpy as np
import scipy.optimize as opt
import matplotlib.pyplot as plt

from stormbird_setup.section_models import Foil as FoilSetup, VaryingFoil as VaryingFoilSetup

from plot_sectional_data import positive_foils, varying_foil_with_mirrored_foils
from suction_sail_data import CQ_VALUES_3D, lift_data
from suction_sail_model import (
    RETUNED_VARYING_FOIL_FILE,
    load_raw_varying_foil,
    create_simulation,
    suction_coefficient_direct,
    angle_sweep,
    run_case,
)

# Data more than this below the angle of attack with the maximum lift is well below stall
WELL_BELOW_STALL_MARGIN_DEG = 10.0

# Number of angles of attack well below stall, and close to stall, used when comparing to the
# simulations
NR_COMPARISON_ANGLES_WELL_BELOW_STALL = 5
NR_COMPARISON_ANGLES_CLOSE_TO_STALL = 6

# Data closer than this to the angle of attack with the maximum lift is part of the stall
# transition, and is not used when tuning the higher order lift term
STALL_TRANSITION_MARGIN_DEG = 3.0

# Stall angle used when tuning the lift, so that the stall does not affect the lift in the range of
# the data
DISABLED_STALL_ANGLE = np.radians(80.0)

# Initial values and limits for the higher order lift term
# The power of the higher order lift term, which is the same for all foil models
HIGH_ORDER_POWER = 3.0

NR_GAUSS_NEWTON_ITERATIONS = 3

# Step sizes for the finite differences
CL_ZERO_ANGLE_STEP = 0.01
CL_INITIAL_SLOPE_FACTOR_STEP = 0.01
HIGH_ORDER_FACTOR_STEP = 0.1

# Angle of attack step when searching for the maximum lift in the simulations
STALL_SEARCH_STEP_DEG = 0.5

# Range for the stall angle offset
STALL_ANGLE_OFFSET_RANGE = np.radians([-15.0, 15.0])

# The stall angle offsets at the different suction coefficients depend slightly on each other, so
# they are tuned more than once
NR_STALL_ANGLE_PASSES = 2


class Data3D:
    '''
    The three-dimensional lift data for one suction coefficient, divided into categories based on
    the angle of attack with the maximum lift, and with a linear curve fitted to the lift well below
    stall.
    '''
    def __init__(self, cq: float):
        self.cq = cq

        self.aoa_deg, self.cl = lift_data(cq, "3d")

        index_max_lift = np.argmax(self.cl)

        self.stall_angle_deg = self.aoa_deg[index_max_lift]
        self.max_lift = self.cl[index_max_lift]

        indices = np.arange(len(self.aoa_deg))

        self.well_below_stall = self.aoa_deg < self.stall_angle_deg - WELL_BELOW_STALL_MARGIN_DEG
        self.close_to_stall = ~self.well_below_stall & (indices <= index_max_lift)
        self.post_stall = indices > index_max_lift

        self.linear_slope, self.linear_intercept = np.polyfit(
            self.aoa_deg[self.well_below_stall], self.cl[self.well_below_stall], 1
        )

        self.angles_well_below_stall_deg = np.linspace(
            self.aoa_deg[self.well_below_stall].min(),
            self.aoa_deg[self.well_below_stall].max(),
            NR_COMPARISON_ANGLES_WELL_BELOW_STALL
        )

        # Data points close to stall, evenly distributed among the available points. The points
        # closest to the maximum lift are part of the stall transition, which is modeled by the
        # stall angle rather than the higher order lift term, and are therefore not used.
        close_to_stall_indices = indices[
            self.close_to_stall & (self.aoa_deg < self.stall_angle_deg - STALL_TRANSITION_MARGIN_DEG)
        ]
        selected = np.linspace(
            0, len(close_to_stall_indices) - 1, NR_COMPARISON_ANGLES_CLOSE_TO_STALL
        ).round().astype(int)

        self.angles_close_to_stall_deg = self.aoa_deg[close_to_stall_indices[selected]]
        self.cl_close_to_stall = self.cl[close_to_stall_indices[selected]]

    def linear_lift(self, aoa_deg: np.ndarray) -> np.ndarray:
        return self.linear_intercept + self.linear_slope * aoa_deg


@dataclass
class Corrections:
    '''
    Corrections to the foil models, given at the suction coefficients with three-dimensional data.
    '''
    cl_zero_angle_offset: np.ndarray
    cl_initial_slope_factor: np.ndarray
    cl_high_order_factor: np.ndarray
    stall_angle_offset: np.ndarray

    @classmethod
    def none(cls) -> "Corrections":
        n = len(CQ_VALUES_3D)

        return cls(np.zeros(n), np.ones(n), np.zeros(n), np.zeros(n))

    def lift_corrections(self) -> np.ndarray:
        return np.concatenate([
            self.cl_zero_angle_offset,
            self.cl_initial_slope_factor,
            self.cl_high_order_factor,
        ])

    def with_lift_corrections(self, x: np.ndarray) -> "Corrections":
        n = len(CQ_VALUES_3D)

        return Corrections(
            np.array(x[:n]),
            np.array(x[n:2 * n]),
            np.array(x[2 * n:3 * n]),
            self.stall_angle_offset.copy()
        )

    def at(self, cq: float) -> tuple[float, float, float, float]:
        '''
        The corrections at a suction coefficient, linearly interpolated between the suction
        coefficients with three-dimensional data.
        '''
        return (
            float(np.interp(cq, CQ_VALUES_3D, self.cl_zero_angle_offset)),
            float(np.interp(cq, CQ_VALUES_3D, self.cl_initial_slope_factor)),
            float(np.interp(cq, CQ_VALUES_3D, self.cl_high_order_factor)),
            float(np.interp(cq, CQ_VALUES_3D, self.stall_angle_offset)),
        )


def corrected_foil(
    foil: FoilSetup,
    corrections: Corrections,
    cq: float,
    disable_stall: bool,
) -> FoilSetup:
    cl_zero_angle_offset, cl_initial_slope_factor, cl_high_order_factor, stall_angle_offset = (
        corrections.at(cq)
    )

    if disable_stall:
        stall_angle_positive = DISABLED_STALL_ANGLE
        stall_angle_negative = DISABLED_STALL_ANGLE
    else:
        stall_angle_positive = foil.mean_positive_stall_angle + stall_angle_offset
        stall_angle_negative = foil.mean_negative_stall_angle + stall_angle_offset

    return foil.model_copy(update = {
        "cl_zero_angle": foil.cl_zero_angle + cl_zero_angle_offset,
        "cl_initial_slope": foil.cl_initial_slope * cl_initial_slope_factor,
        "cl_high_order_factor_positive": cl_high_order_factor,
        "cl_high_order_factor_negative": cl_high_order_factor,
        "cl_high_order_power": HIGH_ORDER_POWER,
        "mean_positive_stall_angle": stall_angle_positive,
        "mean_negative_stall_angle": stall_angle_negative,
    })


def apply_corrections(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    disable_stall: bool = False
) -> VaryingFoilSetup:
    '''
    Applies the corrections to each foil model in the raw varying foil model, at its own suction
    coefficient. A foil model is added for the largest suction coefficient in the
    three-dimensional data, if this is outside the two-dimensional data.
    '''
    cq_values, foils = positive_foils(raw_varying_foil)

    if max(CQ_VALUES_3D) > max(cq_values):
        cq_values.append(max(CQ_VALUES_3D))
        foils.append(foils[-1].model_copy())

    corrected_foils = [
        corrected_foil(foil, corrections, cq, disable_stall) for cq, foil in zip(cq_values, foils)
    ]

    return varying_foil_with_mirrored_foils(cq_values, corrected_foils)


def lift_residuals(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> np.ndarray:
    '''
    The difference between the lift from the simulations, without stall, and the data, for all
    suction coefficients. Well below stall, the linear curve fitted to the data is used, while the
    data points are used close to stall.
    '''
    simulation = create_simulation(
        suction_coefficient_direct(),
        apply_corrections(raw_varying_foil, corrections, disable_stall=True)
    )

    residuals = []

    for data_cq in data:
        angles = np.concatenate([
            data_cq.angles_well_below_stall_deg, data_cq.angles_close_to_stall_deg
        ])

        cl_data = np.concatenate([
            data_cq.linear_lift(data_cq.angles_well_below_stall_deg), data_cq.cl_close_to_stall
        ])

        cl_model = np.array([result.cl for result in angle_sweep(simulation, angles, data_cq.cq)])

        residuals.append(cl_model - cl_data)

    return np.concatenate(residuals)


def tune_lift(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> Corrections:
    '''
    Tunes the lift corrections with the stall disabled, with Gauss-Newton iterations
    '''
    x = corrections.lift_corrections()

    n = len(CQ_VALUES_3D)
    steps = np.array(
        [CL_ZERO_ANGLE_STEP] * n + [CL_INITIAL_SLOPE_FACTOR_STEP] * n + [HIGH_ORDER_FACTOR_STEP] * n
    )

    def rms_error(residuals: np.ndarray) -> float:
        return float(np.sqrt(np.mean(residuals**2)))

    residuals = lift_residuals(raw_varying_foil, corrections.with_lift_corrections(x), data)

    for iteration in range(NR_GAUSS_NEWTON_ITERATIONS):
        print(f"  Iteration {iteration + 1}: RMS lift error = {rms_error(residuals):.4f}")

        jacobian = np.zeros((len(residuals), len(x)))

        for j in range(len(x)):
            x_perturbed = x.copy()
            x_perturbed[j] += steps[j]

            jacobian[:, j] = (
                lift_residuals(raw_varying_foil, corrections.with_lift_corrections(x_perturbed), data) -
                residuals
            ) / steps[j]

        step = -np.linalg.lstsq(jacobian, residuals, rcond=None)[0]

        # A too large higher order term makes the sectional lift decrease before the angle of attack
        # in the data, which gives a sudden drop in the lift from the simulations. The step is
        # therefore reduced until the error decreases.
        for _ in range(6):
            residuals_new = lift_residuals(
                raw_varying_foil, corrections.with_lift_corrections(x + step), data
            )

            if np.all(np.isfinite(residuals_new)) and rms_error(residuals_new) < rms_error(residuals):
                x, residuals = x + step, residuals_new
                break

            step *= 0.5

    print(f"  Final: RMS lift error = {rms_error(residuals):.4f}")

    return corrections.with_lift_corrections(x)


def simulated_maximum_lift(varying_foil: VaryingFoilSetup, data_cq: Data3D) -> float:
    '''
    The maximum lift before stall in the simulations. The angle of attack is increased from well
    below stall until the lift starts to decrease, so that no simulations are done after stall. The
    maximum is refined with a parabola through the points around it, so that it varies smoothly
    with the model parameters.
    '''
    simulation = create_simulation(suction_coefficient_direct(), varying_foil)

    aoa = max(data_cq.stall_angle_deg - 2.0 * WELL_BELOW_STALL_MARGIN_DEG, 0.0)
    max_aoa = data_cq.stall_angle_deg + 2.0 * WELL_BELOW_STALL_MARGIN_DEG

    cl = [run_case(simulation, aoa, data_cq.cq).cl]

    while aoa < max_aoa:
        aoa += STALL_SEARCH_STEP_DEG

        cl.append(run_case(simulation, aoa, data_cq.cq).cl)

        if cl[-1] < cl[-2]:
            break

    if len(cl) < 3 or cl[-1] >= cl[-2]:
        return max(cl)

    y0, y1, y2 = cl[-3], cl[-2], cl[-1]
    denominator = y0 - 2.0 * y1 + y2

    if denominator == 0.0:
        return y1

    return y1 - 0.125 * (y0 - y2)**2 / denominator


def tune_stall_angle(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    index: int,
    data_cq: Data3D
) -> None:
    '''
    Tunes the stall angle offset at one suction coefficient, so that the maximum lift is the same as
    in the data
    '''
    def maximum_lift_error(offset: float) -> float:
        corrections.stall_angle_offset[index] = offset

        varying_foil = apply_corrections(raw_varying_foil, corrections)

        return simulated_maximum_lift(varying_foil, data_cq) - data_cq.max_lift

    lower, upper = STALL_ANGLE_OFFSET_RANGE

    error_lower, error_upper = maximum_lift_error(lower), maximum_lift_error(upper)

    if error_lower * error_upper > 0.0:
        print(f"  Could not match the maximum lift for Cq = {data_cq.cq}, using the closest limit")
        corrections.stall_angle_offset[index] = lower if abs(error_lower) < abs(error_upper) else upper
        return

    corrections.stall_angle_offset[index] = opt.brentq(
        maximum_lift_error, lower, upper, xtol=np.radians(0.02)
    )


def retune_section_model(data: list[Data3D]) -> tuple[VaryingFoilSetup, Corrections]:
    raw_varying_foil = load_raw_varying_foil()

    print("Tuning the lift before stall, with the stall disabled")
    corrections = tune_lift(raw_varying_foil, Corrections.none(), data)

    print("Tuning the stall angle to the maximum lift")
    for _ in range(NR_STALL_ANGLE_PASSES):
        for index, data_cq in enumerate(data):
            tune_stall_angle(raw_varying_foil, corrections, index, data_cq)

    return apply_corrections(raw_varying_foil, corrections), corrections


if __name__ == "__main__":
    # TODO: Remove this when the corrections are updated to the slope reduction (see the top of the
    # file)
    raise SystemExit(
        "retune_section_model.py is not updated to the slope reduction in the foil model yet. "
        "See the TODO at the top of the file."
    )

    data = [Data3D(cq) for cq in CQ_VALUES_3D]

    retuned_varying_foil, corrections = retune_section_model(data)

    RETUNED_VARYING_FOIL_FILE.write_text(retuned_varying_foil.to_json_string())

    print(f"\nRetuned varying foil model written to {RETUNED_VARYING_FOIL_FILE.name}")
    print(f"\nPower of the higher order lift term: {HIGH_ORDER_POWER}\n")

    print(
        f"{'Cq':>6} {'cl_zero_angle offset':>21} {'cl_initial_slope factor':>24} "
        f"{'high order factor':>18} {'stall offset [deg]':>19} {'max CL data':>12} {'max CL model':>13}"
    )

    for data_cq in data:
        cl_zero_angle_offset, cl_initial_slope_factor, cl_high_order_factor, stall_angle_offset = (
            corrections.at(data_cq.cq)
        )

        max_lift_model = simulated_maximum_lift(retuned_varying_foil, data_cq)

        print(
            f"{data_cq.cq:6.3f} {cl_zero_angle_offset:21.3f} {cl_initial_slope_factor:24.3f} "
            f"{cl_high_order_factor:18.3f} {np.degrees(stall_angle_offset):19.2f} "
            f"{data_cq.max_lift:12.2f} {max_lift_model:13.2f}"
        )

    w_plot = 18
    h_plot = w_plot / 3.2

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_data = fig.add_subplot(131)
    ax_lift_corrections = fig.add_subplot(132)
    ax_stall_correction = fig.add_subplot(133)

    # The data in each category, the linear curve fitted to the data well below stall, and the
    # simulated lift without stall
    raw_varying_foil = load_raw_varying_foil()

    simulation_without_stall = create_simulation(
        suction_coefficient_direct(),
        apply_corrections(raw_varying_foil, corrections, disable_stall=True)
    )

    for data_cq in data:
        color = ax_data.plot(
            data_cq.aoa_deg[data_cq.well_below_stall],
            data_cq.cl[data_cq.well_below_stall],
            "o",
            markersize=3,
            label=f"Cq = {data_cq.cq}, well below stall"
        )[0].get_color()

        ax_data.plot(
            data_cq.aoa_deg[data_cq.close_to_stall], data_cq.cl[data_cq.close_to_stall],
            "^", markersize=3, color=color, label=f"Cq = {data_cq.cq}, close to stall"
        )
        ax_data.plot(
            data_cq.aoa_deg[data_cq.post_stall], data_cq.cl[data_cq.post_stall],
            "x", markersize=3, color=color, label=f"Cq = {data_cq.cq}, after stall"
        )

        aoa_line = np.array([0.0, data_cq.stall_angle_deg])
        ax_data.plot(aoa_line, data_cq.linear_lift(aoa_line), ":", color=color)

        aoa_model = np.arange(0.0, data_cq.stall_angle_deg + 0.5, 1.0)
        cl_model = [result.cl for result in angle_sweep(simulation_without_stall, aoa_model, data_cq.cq)]

        ax_data.plot(aoa_model, cl_model, "-", color=color)

    ax_data.set_title("Data, linear fit (dotted), and lifting line without stall (solid)")
    ax_data.set_xlabel("Angle of attack [deg]")
    ax_data.set_ylabel("Lift coefficient (CL)")
    ax_data.legend(fontsize=7)

    # The corrections as a function of the suction coefficient, with the values applied to each foil
    # model as points
    cq_foils, _ = positive_foils(retuned_varying_foil)
    cq_line = np.linspace(0.0, max(cq_foils), 100)

    corrections_line = np.array([corrections.at(cq) for cq in cq_line])
    corrections_foils = np.array([corrections.at(cq) for cq in cq_foils])

    labels = ["Offset to cl_zero_angle", "Factor on cl_initial_slope", "High order lift factor"]

    for i, label in enumerate(labels):
        color = f"C{i}"

        ax_lift_corrections.plot(cq_line, corrections_line[:, i], "-", color=color, label=label)
        ax_lift_corrections.plot(cq_foils, corrections_foils[:, i], "o", color=color)

    ax_lift_corrections.set_title("Lift corrections")
    ax_lift_corrections.set_xlabel("Suction coefficient (CQ)")
    ax_lift_corrections.legend()

    ax_stall_correction.plot(cq_line, np.degrees(corrections_line[:, 3]), "-", color="C3")
    ax_stall_correction.plot(cq_foils, np.degrees(corrections_foils[:, 3]), "o", color="C3")

    ax_stall_correction.set_title("Stall angle correction")
    ax_stall_correction.set_xlabel("Suction coefficient (CQ)")
    ax_stall_correction.set_ylabel("Stall angle offset [deg]")

    plt.tight_layout()
    plt.show()
