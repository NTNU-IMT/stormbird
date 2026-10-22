'''
Optional step in the suction sail example: retunes the section model from the first step, so that
the three-dimensional simulations match the three-dimensional data better.

The following parameters are tuned:

- The lift at zero angle of attack, `cl_zero_angle`, with an offset, and the lift slope,
  `cl_initial_slope`, with a factor. These are tuned to match a linear fit of the three-dimensional
  data well below stall, which avoids noise in the data close to stall.

- The slope reduction parameters, `cl_slope_reduction_factor` and `cl_slope_reduction_onset_angle`,
  are tuned to match the lift data close to stall (but before the stall transition). This captures
  the bending of the lift curve before stall. The correction is applied to the positive or negative
  side based on the sign of `cl_zero_angle`.

- The minimum drag, `cd_min`, is tuned by comparing the total drag from simulations (profile drag
  plus induced drag) to the lowest drag points in the three-dimensional data.

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

import matplotlib.pyplot as plt
import numpy as np
from stormbird_setup.section_models import Foil as FoilSetup, VaryingFoil as VaryingFoilSetup

from plot_sectional_data import positive_foils, varying_foil_with_mirrored_foils
from suction_sail_data import CQ_VALUES_3D, drag_data, lift_data
from suction_sail_model import (
    RETUNED_VARYING_FOIL_FILE,
    angle_sweep,
    create_simulation,
    load_raw_varying_foil,
    suction_coefficient_direct,
)

# Data more than this below the angle of attack with the maximum lift is well below stall
WELL_BELOW_STALL_MARGIN_DEG = 10.0

# Data closer than this to the angle of attack with the maximum lift is part of the stall
# transition, and is not used when tuning the slope reduction
STALL_TRANSITION_MARGIN_DEG = 3.0

# Number of angles of attack well below stall used when comparing to the simulations
NR_COMPARISON_ANGLES_WELL_BELOW_STALL = 5

# Number of angles of attack close to stall used when tuning the slope reduction
NR_COMPARISON_ANGLES_CLOSE_TO_STALL = 5

# Step sizes for the finite differences
CL_ZERO_ANGLE_STEP = 0.01
CL_INITIAL_SLOPE_FACTOR_STEP = 0.01
SLOPE_REDUCTION_FACTOR_STEP = 0.01
SLOPE_REDUCTION_ONSET_ANGLE_STEP = 0.01  # radians

NR_GAUSS_NEWTON_ITERATIONS = 3
NR_SLOPE_REDUCTION_ITERATIONS = 3

# Number of lowest drag points to use when tuning cd_min
NR_DRAG_TUNING_POINTS = 3

# Stall angle used when tuning the slope reduction, so that the stall does not interfere
DISABLED_STALL_ANGLE = np.radians(80.0)


class Data3D:
    '''
    The three-dimensional data for one suction coefficient. A linear curve is fitted to the lift
    data well below stall, which is used when tuning the linear lift parameters.
    '''
    def __init__(self, cq: float):
        self.cq = cq

        aoa_deg, cl = lift_data(cq, "3d")
        cl_drag, cd = drag_data(cq, "3d")

        index_max_lift = np.argmax(cl)
        self.stall_angle_deg = aoa_deg[index_max_lift]

        well_below_stall = aoa_deg < self.stall_angle_deg - WELL_BELOW_STALL_MARGIN_DEG

        self.linear_slope, self.linear_intercept = np.polyfit(
            aoa_deg[well_below_stall], cl[well_below_stall], 1
        )

        self.angles_well_below_stall_deg = np.linspace(
            aoa_deg[well_below_stall].min(),
            aoa_deg[well_below_stall].max(),
            NR_COMPARISON_ANGLES_WELL_BELOW_STALL
        )

        # Data points close to stall but before the stall transition, used for tuning the slope
        # reduction. These are between "well below stall" and "stall transition".
        indices = np.arange(len(aoa_deg))
        close_to_stall = (
            (aoa_deg >= self.stall_angle_deg - WELL_BELOW_STALL_MARGIN_DEG) &
            (aoa_deg < self.stall_angle_deg - STALL_TRANSITION_MARGIN_DEG) &
            (indices <= index_max_lift)
        )

        if np.sum(close_to_stall) >= NR_COMPARISON_ANGLES_CLOSE_TO_STALL:
            # Select evenly distributed points from the close-to-stall region
            close_to_stall_indices = indices[close_to_stall]
            selected = np.linspace(
                0, len(close_to_stall_indices) - 1, NR_COMPARISON_ANGLES_CLOSE_TO_STALL
            ).round().astype(int)
            self.angles_close_to_stall_deg = aoa_deg[close_to_stall_indices[selected]]
            self.cl_close_to_stall = cl[close_to_stall_indices[selected]]
        else:
            # Not enough points, use what we have
            self.angles_close_to_stall_deg = aoa_deg[close_to_stall]
            self.cl_close_to_stall = cl[close_to_stall]

        # The points with the lowest drag from the drag polar, used for tuning cd_min.
        # We use multiple points to get a more robust estimate.
        sorted_indices = np.argsort(cd)[:NR_DRAG_TUNING_POINTS]
        self.cd_tuning_points = cd[sorted_indices]
        self.cl_tuning_points = cl_drag[sorted_indices]

    def linear_lift(self, aoa_deg: np.ndarray) -> np.ndarray:
        return self.linear_intercept + self.linear_slope * aoa_deg


@dataclass
class Corrections:
    '''
    Corrections to the foil models, given at the suction coefficients with three-dimensional data.
    '''
    cl_zero_angle_offset: np.ndarray
    cl_initial_slope_factor: np.ndarray
    cl_slope_reduction_factor: np.ndarray
    cl_slope_reduction_onset_angle: np.ndarray
    cd_min_offset: np.ndarray

    @classmethod
    def none(cls) -> "Corrections":
        n = len(CQ_VALUES_3D)
        return cls(
            np.zeros(n),       # cl_zero_angle_offset
            np.ones(n),        # cl_initial_slope_factor
            np.zeros(n),       # cl_slope_reduction_factor (0 = no reduction)
            np.zeros(n),       # cl_slope_reduction_onset_angle
            np.zeros(n),       # cd_min_offset
        )

    def linear_lift_as_array(self) -> np.ndarray:
        return np.concatenate([
            self.cl_zero_angle_offset,
            self.cl_initial_slope_factor,
        ])

    def with_linear_lift_values(self, x: np.ndarray) -> "Corrections":
        n = len(CQ_VALUES_3D)
        return Corrections(
            np.array(x[:n]),
            np.array(x[n:2 * n]),
            self.cl_slope_reduction_factor.copy(),
            self.cl_slope_reduction_onset_angle.copy(),
            self.cd_min_offset.copy(),
        )

    def slope_reduction_as_array(self) -> np.ndarray:
        return np.concatenate([
            self.cl_slope_reduction_factor,
            self.cl_slope_reduction_onset_angle,
        ])

    def with_slope_reduction_values(self, x: np.ndarray) -> "Corrections":
        n = len(CQ_VALUES_3D)
        return Corrections(
            self.cl_zero_angle_offset.copy(),
            self.cl_initial_slope_factor.copy(),
            np.array(x[:n]),
            np.array(x[n:2 * n]),
            self.cd_min_offset.copy(),
        )

    def at(self, cq: float) -> tuple[float, float, float, float, float]:
        '''
        The corrections at a suction coefficient, linearly interpolated between the suction
        coefficients with three-dimensional data.
        '''
        return (
            float(np.interp(cq, CQ_VALUES_3D, self.cl_zero_angle_offset)),
            float(np.interp(cq, CQ_VALUES_3D, self.cl_initial_slope_factor)),
            float(np.interp(cq, CQ_VALUES_3D, self.cl_slope_reduction_factor)),
            float(np.interp(cq, CQ_VALUES_3D, self.cl_slope_reduction_onset_angle)),
            float(np.interp(cq, CQ_VALUES_3D, self.cd_min_offset)),
        )


def corrected_foil(
    foil: FoilSetup,
    corrections: Corrections,
    cq: float,
    disable_stall: bool = False
) -> FoilSetup:
    '''
    Applies the corrections to a foil model at a given suction coefficient.
    
    The slope reduction is applied to the positive or negative side based on the sign of
    cl_zero_angle. With positive cl_zero_angle (suction on the upper surface), the operating
    angles of attack are typically positive, so we tune the positive slope reduction.
    
    If disable_stall is True, the stall angles are set to a very large value so that stall
    does not interfere with the slope reduction tuning.
    '''
    (
        cl_zero_angle_offset,
        cl_initial_slope_factor,
        cl_slope_reduction_factor,
        cl_slope_reduction_onset_angle,
        cd_min_offset
    ) = corrections.at(cq)

    # The foil parameters should always be set when coming from the fitted model
    assert foil.cl_zero_angle is not None
    assert foil.cl_initial_slope is not None
    assert foil.cd_min is not None

    updates: dict[str, float] = {
        "cl_zero_angle": foil.cl_zero_angle + cl_zero_angle_offset,
        "cl_initial_slope": foil.cl_initial_slope * cl_initial_slope_factor,
        "cd_min": foil.cd_min + cd_min_offset,
    }

    # Disable stall by setting very large stall angles
    if disable_stall:
        updates["mean_positive_stall_angle"] = DISABLED_STALL_ANGLE
        updates["mean_negative_stall_angle"] = DISABLED_STALL_ANGLE

    # Apply slope reduction to the appropriate side based on cl_zero_angle sign
    corrected_cl_zero_angle = foil.cl_zero_angle + cl_zero_angle_offset

    if corrected_cl_zero_angle >= 0:
        # Positive cl_zero_angle: tune positive side
        base_onset = foil.cl_slope_reduction_onset_angle_positive or 0.0
        updates["cl_slope_reduction_factor_positive"] = cl_slope_reduction_factor
        updates["cl_slope_reduction_onset_angle_positive"] = base_onset + cl_slope_reduction_onset_angle
    else:
        # Negative cl_zero_angle: tune negative side
        base_onset = foil.cl_slope_reduction_onset_angle_negative or 0.0
        updates["cl_slope_reduction_factor_negative"] = cl_slope_reduction_factor
        updates["cl_slope_reduction_onset_angle_negative"] = base_onset + cl_slope_reduction_onset_angle

    return foil.model_copy(update=updates)


def apply_corrections(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    disable_stall: bool = False,
) -> VaryingFoilSetup:
    '''
    Applies the corrections to each foil model in the raw varying foil model, at its own suction
    coefficient. A foil model is added for the largest suction coefficient in the
    three-dimensional data, if this is outside the two-dimensional data.
    
    If disable_stall is True, the stall angles are set to a very large value.
    '''
    cq_values, foils = positive_foils(raw_varying_foil)

    if max(CQ_VALUES_3D) > max(cq_values):
        cq_values.append(max(CQ_VALUES_3D))
        foils.append(foils[-1].model_copy())

    corrected_foils = [
        corrected_foil(foil, corrections, cq, disable_stall) for cq, foil in zip(cq_values, foils)
    ]

    return varying_foil_with_mirrored_foils(cq_values, corrected_foils)


def linear_lift_residuals(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> np.ndarray:
    '''
    The difference between the lift from the simulations and the linear fit of the data well below
    stall, for all suction coefficients.
    '''
    simulation = create_simulation(
        suction_coefficient_direct(),
        apply_corrections(raw_varying_foil, corrections)
    )

    residuals = []

    for data_cq in data:
        angles = data_cq.angles_well_below_stall_deg
        cl_data = data_cq.linear_lift(angles)
        cl_model = np.array([result.cl for result in angle_sweep(simulation, angles, data_cq.cq)])

        residuals.append(cl_model - cl_data)

    return np.concatenate(residuals)


def slope_reduction_residuals(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> np.ndarray:
    '''
    The difference between the lift from the simulations and the data close to stall,
    for all suction coefficients. Stall is disabled so the slope reduction effect can be
    tuned without interference from stall.
    '''
    simulation = create_simulation(
        suction_coefficient_direct(),
        apply_corrections(raw_varying_foil, corrections, disable_stall=True)
    )

    residuals = []

    for data_cq in data:
        if len(data_cq.angles_close_to_stall_deg) == 0:
            continue

        angles = data_cq.angles_close_to_stall_deg
        cl_data = data_cq.cl_close_to_stall
        cl_model = np.array([result.cl for result in angle_sweep(simulation, angles, data_cq.cq)])

        residuals.append(cl_model - cl_data)

    return np.concatenate(residuals) if residuals else np.array([])


def tune_linear_lift(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> Corrections:
    '''
    Tunes the linear lift corrections using Gauss-Newton iterations.
    '''
    x = corrections.linear_lift_as_array()

    n = len(CQ_VALUES_3D)
    steps = np.array([CL_ZERO_ANGLE_STEP] * n + [CL_INITIAL_SLOPE_FACTOR_STEP] * n)

    def rms_error(residuals: np.ndarray) -> float:
        return float(np.sqrt(np.mean(residuals**2)))

    residuals = linear_lift_residuals(raw_varying_foil, corrections.with_linear_lift_values(x), data)

    for iteration in range(NR_GAUSS_NEWTON_ITERATIONS):
        print(f"  Iteration {iteration + 1}: RMS lift error = {rms_error(residuals):.4f}")

        jacobian = np.zeros((len(residuals), len(x)))

        for j in range(len(x)):
            x_perturbed = x.copy()
            x_perturbed[j] += steps[j]

            jacobian[:, j] = (
                linear_lift_residuals(raw_varying_foil, corrections.with_linear_lift_values(x_perturbed), data) -
                residuals
            ) / steps[j]

        step = -np.linalg.lstsq(jacobian, residuals, rcond=None)[0]

        # Reduce the step if needed to ensure the error decreases
        for _ in range(6):
            residuals_new = linear_lift_residuals(
                raw_varying_foil, corrections.with_linear_lift_values(x + step), data
            )

            if np.all(np.isfinite(residuals_new)) and rms_error(residuals_new) < rms_error(residuals):
                x, residuals = x + step, residuals_new
                break

            step *= 0.5

    print(f"  Final: RMS lift error = {rms_error(residuals):.4f}")

    return corrections.with_linear_lift_values(x)


def tune_slope_reduction(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> Corrections:
    '''
    Tunes the slope reduction parameters using Gauss-Newton iterations to match the lift
    data close to stall.
    '''
    x = corrections.slope_reduction_as_array()

    n = len(CQ_VALUES_3D)
    steps = np.array([SLOPE_REDUCTION_FACTOR_STEP] * n + [SLOPE_REDUCTION_ONSET_ANGLE_STEP] * n)

    def rms_error(residuals: np.ndarray) -> float:
        return float(np.sqrt(np.mean(residuals**2))) if len(residuals) > 0 else 0.0

    residuals = slope_reduction_residuals(raw_varying_foil, corrections.with_slope_reduction_values(x), data)

    if len(residuals) == 0:
        print("  No close-to-stall data available for slope reduction tuning")
        return corrections

    for iteration in range(NR_SLOPE_REDUCTION_ITERATIONS):
        print(f"  Iteration {iteration + 1}: RMS lift error (close to stall) = {rms_error(residuals):.4f}")

        jacobian = np.zeros((len(residuals), len(x)))

        for j in range(len(x)):
            x_perturbed = x.copy()
            x_perturbed[j] += steps[j]

            jacobian[:, j] = (
                slope_reduction_residuals(raw_varying_foil, corrections.with_slope_reduction_values(x_perturbed), data) -
                residuals
            ) / steps[j]

        step = -np.linalg.lstsq(jacobian, residuals, rcond=None)[0]

        # Reduce the step if needed to ensure the error decreases
        for _ in range(6):
            residuals_new = slope_reduction_residuals(
                raw_varying_foil, corrections.with_slope_reduction_values(x + step), data
            )

            if np.all(np.isfinite(residuals_new)) and rms_error(residuals_new) < rms_error(residuals):
                x, residuals = x + step, residuals_new
                break

            step *= 0.5

    print(f"  Final: RMS lift error (close to stall) = {rms_error(residuals):.4f}")

    return corrections.with_slope_reduction_values(x)


def tune_cd_min(
    raw_varying_foil: VaryingFoilSetup,
    corrections: Corrections,
    data: list[Data3D]
) -> Corrections:
    '''
    Tunes cd_min by comparing the total drag from simulations (profile + induced) to the drag
    in the 3D data at multiple low-drag points. The offset minimizes the mean error across these
    points, giving a more robust estimate than using a single point.
    '''
    simulation = create_simulation(
        suction_coefficient_direct(),
        apply_corrections(raw_varying_foil, corrections)
    )

    angles_of_attack_deg = np.arange(0.0, 30.0, 0.5)

    for i, data_cq in enumerate(data):
        # Run an angle sweep to find the drag at different lift coefficients
        results = angle_sweep(simulation, angles_of_attack_deg, data_cq.cq)

        cl_sim = np.array([r.cl for r in results])
        cd_sim = np.array([r.cd for r in results])

        # Interpolate to find the simulated drag at each tuning point's lift coefficient
        cd_sim_at_points = np.interp(data_cq.cl_tuning_points, cl_sim, cd_sim)

        # The offset that minimizes the mean error across all tuning points
        offsets = data_cq.cd_tuning_points - cd_sim_at_points
        corrections.cd_min_offset[i] = np.mean(offsets)

        print(f"  Cq = {data_cq.cq:.3f}: cd_data = {data_cq.cd_tuning_points}, "
              f"cd_sim = {cd_sim_at_points}, mean offset = {corrections.cd_min_offset[i]:.4f}")

    return corrections


def retune_section_model(data: list[Data3D]) -> tuple[VaryingFoilSetup, Corrections]:
    raw_varying_foil = load_raw_varying_foil()

    print("Tuning the linear lift parameters")
    corrections = tune_linear_lift(raw_varying_foil, Corrections.none(), data)

    print("\nTuning the slope reduction (lift close to stall)")
    corrections = tune_slope_reduction(raw_varying_foil, corrections, data)

    print("\nTuning the minimum drag")
    corrections = tune_cd_min(raw_varying_foil, corrections, data)

    return apply_corrections(raw_varying_foil, corrections), corrections


if __name__ == "__main__":
    data = [Data3D(cq) for cq in CQ_VALUES_3D]

    print("Three-dimensional data:")
    print(f"{'Cq':>6} {'slope':>8} {'intercept':>10} {'stall angle':>12}")
    for d in data:
        print(f"{d.cq:6.3f} {d.linear_slope:8.4f} {d.linear_intercept:10.4f} {d.stall_angle_deg:12.1f}")
        print(f"       close-to-stall angles: {d.angles_close_to_stall_deg}")
        print(f"       close-to-stall cl:     {d.cl_close_to_stall}")

    print()

    retuned_varying_foil, corrections = retune_section_model(data)

    RETUNED_VARYING_FOIL_FILE.write_text(retuned_varying_foil.to_json_string())

    print(f"\nRetuned varying foil model written to {RETUNED_VARYING_FOIL_FILE.name}")

    print("\nCorrections at each suction coefficient:")
    print(f"{'Cq':>6} {'cl_0 off':>9} {'slope fac':>10} {'red fac':>8} {'red onset':>10} {'cd_min off':>11}")
    for cq, cl_0, slope, red_fac, red_onset, cd_min in zip(
        CQ_VALUES_3D,
        corrections.cl_zero_angle_offset,
        corrections.cl_initial_slope_factor,
        corrections.cl_slope_reduction_factor,
        corrections.cl_slope_reduction_onset_angle,
        corrections.cd_min_offset,
    ):
        print(f"{cq:6.3f} {cl_0:9.4f} {slope:10.4f} {red_fac:8.4f} {np.degrees(red_onset):10.2f}° {cd_min:11.4f}")

    # Plot the results
    angles_of_attack_deg = np.arange(0.0, 45.5, 0.5)

    raw_varying_foil = load_raw_varying_foil()

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize=(w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    for data_cq in data:
        cq = data_cq.cq

        # Data
        aoa_data, cl_data = lift_data(cq, "3d")
        cl_drag_data, cd_data = drag_data(cq, "3d")

        color = ax_cl.plot(aoa_data, cl_data, "o", markersize=3, label=f"Data, Cq = {cq}")[0].get_color()
        ax_cd.plot(cl_drag_data, cd_data, "o", markersize=3, color=color)

        # Linear fit used for tuning
        aoa_linear = np.array([0.0, data_cq.angles_well_below_stall_deg.max()])
        ax_cl.plot(aoa_linear, data_cq.linear_lift(aoa_linear), ":", color=color, linewidth=1)

        # Raw model (fitted to 2D data)
        simulation_raw = create_simulation(suction_coefficient_direct(), raw_varying_foil)
        results_raw = angle_sweep(simulation_raw, angles_of_attack_deg, cq)
        cl_raw = [r.cl for r in results_raw]
        cd_raw = [r.cd for r in results_raw]

        ax_cl.plot(angles_of_attack_deg, cl_raw, "--", color=color, alpha=0.5)
        ax_cd.plot(cl_raw, cd_raw, "--", color=color, alpha=0.5)

        # Retuned model
        simulation_retuned = create_simulation(suction_coefficient_direct(), retuned_varying_foil)
        results_retuned = angle_sweep(simulation_retuned, angles_of_attack_deg, cq)
        cl_retuned = [r.cl for r in results_retuned]
        cd_retuned = [r.cd for r in results_retuned]

        ax_cl.plot(angles_of_attack_deg, cl_retuned, "-", color=color)
        ax_cd.plot(cl_retuned, cd_retuned, "-", color=color)

    # Legend entries for line styles
    ax_cl.plot([], [], "--", color="gray", alpha=0.5, label="Fitted to 2D data")
    ax_cl.plot([], [], "-", color="gray", label="Retuned to 3D data")
    ax_cl.plot([], [], ":", color="gray", linewidth=1, label="Linear fit (tuning target)")

    ax_cl.legend()

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.tight_layout()
    plt.show()
