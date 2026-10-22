'''
Step 3 in the suction sail example: tunes the parameters of the internal flow model, so that the
`PowerToInternalFlow` conversion matches the power curve for the three-dimensional model in the
paper.

The power curve gives the lift coefficient as a function of the power coefficient, but not the
angle of attack or the suction coefficient used for each point. It is therefore assumed that the
lowest suction coefficient that can give the lift was used for each point, which means that the
sail is operated close to the maximum lift for the suction coefficient. A small margin to the
maximum lift is used, to avoid operating exactly at stall in the simulations.

An alternative assumption could be to use the combination of angle of attack and suction
coefficient that minimizes the drag for each lift coefficient. However, more suction always gives
lower drag for the same lift in the data, so this assumption gives the largest suction coefficient
for most points, independent of the power.

The porous fraction and the fan efficiency are assumed, while the loss coefficient and the two
coefficients for the external pressure are tuned.
'''

from pathlib import Path

import numpy as np
import scipy.optimize as opt
import matplotlib.pyplot as plt

from stormbird_setup.section_models import ActiveAirFlowFoil, InternalFlow
from pystormbird.section_models import ActiveAirFlowFoil as ActiveAirFlowFoilModel

from suction_sail_data import power_curve_data
from suction_sail_model import (
    AREA, DENSITY, VELOCITY,
    create_simulation,
    suction_coefficient_direct,
    power_to_internal_flow,
    load_section_varying_foil,
    angle_sweep,
    run_case,
    first_maximum,
    SimulationResult,
)

POROUS_FRACTION = 0.25
FAN_EFFICIENCY = 0.7

# Margin to the maximum lift for each suction coefficient, as a fraction of the maximum lift
LIFT_MARGIN = 0.03

# Only points with a power coefficient below this value are used when tuning. The paper states that
# the power coefficient must not exceed 0.2 for naval applications. For larger power coefficients,
# the lift in the data increases very little with the power, which is not captured by the section
# model based on the two-dimensional data, as the comparison in step 2 shows.
MAX_POWER_COEFFICIENT_FOR_TUNING = 0.25

TUNED_SECTION_MODEL_FILE = Path(__file__).parent / "tuned_active_air_flow_foil.json"

# The data can not separate the effect of the lift on the external pressure (`cp_gradient`) from the
# effect of the suction coefficient, as both increase along the power curve. With the section model
# fitted to the two-dimensional data, tuning all three parameters ends with a loss coefficient of
# zero, which is not physical, and gives no flow through the porous surface. `cp_gradient` is
# therefore fixed at zero, which gives practically the same error in the power.
# TODO: Check if `cp_gradient` can be tuned again when the retuned section model is updated (see
# retune_section_model.py)
TUNE_CP_GRADIENT = False


class OperatingPoint:
    '''
    The assumed angle of attack and suction coefficient for a point on the power curve, together
    with the flow on each line element from a simulation with this suction coefficient.
    '''
    def __init__(
        self,
        *,
        cp_data: float,
        cl_data: float,
        cq: float,
        angle_of_attack_deg: float,
        result: SimulationResult
    ):
        self.cp_data = cp_data
        self.cl_data = cl_data
        self.cq = cq
        self.angle_of_attack_deg = angle_of_attack_deg
        self.result = result


def maximum_lift_per_suction_coefficient(
    cq_values: np.ndarray,
    angles_of_attack_deg: np.ndarray
) -> np.ndarray:
    '''
    The maximum lift before stall for each suction coefficient, with the suction coefficient set
    directly on the wing.
    '''
    simulation = create_simulation(suction_coefficient_direct(), load_section_varying_foil())

    cl_max = np.zeros(len(cq_values))

    for i, cq in enumerate(cq_values):
        cl = np.array([result.cl for result in angle_sweep(simulation, angles_of_attack_deg, cq)])

        cl_max[i] = cl[first_maximum(cl)]

    return cl_max


def find_operating_points(angles_of_attack_deg: np.ndarray) -> list[OperatingPoint]:
    '''
    Finds the operating point for each point on the power curve, assuming that the lowest suction
    coefficient that gives the lift is used. Points where no suction is needed in the model are
    skipped.
    '''
    # The range of suction coefficients in the section model
    cq_values = np.linspace(0.0, max(load_section_varying_foil().internal_state_data), 20)

    cl_max = maximum_lift_per_suction_coefficient(cq_values, angles_of_attack_deg)

    cl_usable = (1.0 - LIFT_MARGIN) * np.maximum.accumulate(cl_max)

    simulation = create_simulation(suction_coefficient_direct(), load_section_varying_foil())

    operating_points = []

    for cp, cl in zip(*power_curve_data()):
        if cl <= cl_usable[0]:
            print(f"Skipping the point with CL = {cl:.2f}, as no suction is needed in the model")
            continue

        if cl > cl_usable[-1]:
            print(
                f"Skipping the point with CL = {cl:.2f}, as it is above the maximum lift in the "
                "model, with the margin to stall"
            )
            continue

        cq = np.interp(cl, cl_usable, cq_values)

        cl_sweep = np.array([
            result.cl for result in angle_sweep(simulation, angles_of_attack_deg, cq)
        ])

        index_max = first_maximum(cl_sweep)

        angle_of_attack_deg = np.interp(
            cl, cl_sweep[:index_max + 1], angles_of_attack_deg[:index_max + 1]
        )

        operating_points.append(OperatingPoint(
            cp_data = cp,
            cl_data = cl,
            cq = cq,
            angle_of_attack_deg = angle_of_attack_deg,
            result = run_case(simulation, angle_of_attack_deg, cq)
        ))

    return operating_points


def tune_internal_flow(operating_points: list[OperatingPoint]) -> InternalFlow:
    '''
    Tunes the internal flow model to the power in the operating points.

    With a given suction coefficient on each line element, the power coefficient of the model is
    linear in the three unknown parameters:

    C_P = sum_i (C_Q / eta) (K C_Q^2 / phi^2 - cp_zero_cl - cp_gradient C_L,i) w_i

    where w_i = A_i U_i^3 / (A U^3) is the weight of each line element. The parameters are therefore
    found with linear least squares on the relative error in the power, using the suction
    coefficient and the lift on each line element from the simulations with a constant suction
    coefficient along the span.
    '''
    foil_model = ActiveAirFlowFoilModel(
        ActiveAirFlowFoil(
            varying_foil = load_section_varying_foil(),
            internal_state_conversion = suction_coefficient_direct()
        ).to_json_string()
    )

    rows = []

    for point in operating_points:
        cq = point.cq

        nr_sections = len(point.result.section_velocities)

        weights = (point.result.section_velocities / VELOCITY)**3 / nr_sections

        section_cl = np.array([
            foil_model.lift_coefficient(angle, cq) for angle in point.result.section_angles_of_attack
        ])

        row = np.array([
            cq**3 * np.sum(weights) / POROUS_FRACTION**2,
            -cq * np.sum(weights),
            -cq * np.sum(weights * section_cl),
        ]) / FAN_EFFICIENCY

        rows.append(row / point.cp_data)

    # The loss coefficient must be positive, while the external pressure on the suction side is
    # assumed to be negative, and to decrease with increasing lift
    if TUNE_CP_GRADIENT:
        fit = opt.lsq_linear(
            np.array(rows),
            np.ones(len(rows)),
            bounds = ([0.0, -np.inf, -np.inf], [np.inf, 0.0, 0.0])
        )

        loss_coefficient, cp_zero_cl, cp_gradient = fit.x
    else:
        fit = opt.lsq_linear(
            np.array(rows)[:, :2],
            np.ones(len(rows)),
            bounds = ([0.0, -np.inf], [np.inf, 0.0])
        )

        loss_coefficient, cp_zero_cl = fit.x
        cp_gradient = 0.0

    return InternalFlow(
        cp_zero_cl = cp_zero_cl,
        cp_gradient = cp_gradient,
        loss_coefficient = loss_coefficient,
        porous_fraction = POROUS_FRACTION,
    )


if __name__ == "__main__":
    angles_of_attack_deg = np.arange(0.0, 45.5, 0.5)

    all_points = find_operating_points(angles_of_attack_deg)

    tuning_points = [
        point for point in all_points if point.cp_data <= MAX_POWER_COEFFICIENT_FOR_TUNING
    ]

    internal_flow = tune_internal_flow(tuning_points)

    print("\nTuned internal flow model:")
    print(f"  loss_coefficient = {internal_flow.loss_coefficient:.3f}")
    print(f"  cp_zero_cl       = {internal_flow.cp_zero_cl:.3f}")
    print(f"  cp_gradient      = {internal_flow.cp_gradient:.3f}")

    conversion = power_to_internal_flow(internal_flow, FAN_EFFICIENCY)

    TUNED_SECTION_MODEL_FILE.write_text(
        ActiveAirFlowFoil(
            varying_foil = load_section_varying_foil(),
            internal_state_conversion = conversion
        ).to_json_string()
    )

    print(f"\nTuned section model written to {TUNED_SECTION_MODEL_FILE.name}")

    # Simulate the operating points with the power as the internal state, where the suction
    # coefficient on each line element is found from the internal flow model
    simulation = create_simulation(conversion, load_section_varying_foil())

    power_factor = 0.5 * DENSITY * AREA * VELOCITY**3

    full_model_results = [
        run_case(simulation, point.angle_of_attack_deg, point.cp_data * power_factor)
        for point in all_points
    ]

    # The assumed suction coefficient is constant along the span, as used when tuning, while the
    # model value is the mean over the span from the internal flow model. Similar values indicate
    # that the assumption of a constant suction coefficient when tuning is reasonable.
    print(f"\n{'Cp data':>8} {'CL data':>8} {'CL model':>9} {'Cq assumed':>11} {'Cq model':>9} {'aoa':>6}")

    for point, result in zip(all_points, full_model_results):
        print(
            f"{point.cp_data:8.3f} {point.cl_data:8.2f} {result.cl:9.2f} {point.cq:11.4f} "
            f"{result.wing_suction_coefficient():9.4f} {point.angle_of_attack_deg:6.1f}"
        )

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_span = fig.add_subplot(122)

    cp_data, cl_data = power_curve_data()

    ax_cl.set_title("Lift as a function of power, compared to the data")

    ax_cl.plot(cp_data, cl_data, "o", label="Data")
    ax_cl.plot(
        [point.cp_data for point in all_points],
        [result.cl for result in full_model_results],
        "-",
        label="Lifting line, tuned model"
    )
    ax_cl.axvline(MAX_POWER_COEFFICIENT_FOR_TUNING, color="gray", linestyle="--", label="Tuning limit")

    ax_cl.set_xlabel("Power coefficient (CP)")
    ax_cl.set_ylabel("Lift coefficient (CL)")
    ax_cl.legend()

    # Illustration of the internal flow model, and not a comparison with data: all line elements
    # share the same internal volume, and therefore the same internal suction. The suction
    # coefficient on each line element is then given by the local flow conditions, and therefore
    # varies along the span, even though the power is set for the whole wing.
    ax_span.set_title(
        "Illustration: suction along the span with the tuned model\n"
        "(one internal volume, so the suction depends on the local flow)"
    )

    for point, result in list(zip(all_points, full_model_results))[1:9:3]:
        span_location = (np.arange(len(result.section_internal_states)) + 0.5) / len(result.section_internal_states)

        ax_span.plot(span_location, result.section_internal_states, label=f"CP = {point.cp_data:.3f}")

    ax_span.set_xlabel("Non-dimensional distance from the wall")
    ax_span.set_ylabel("Suction coefficient (CQ)")
    ax_span.legend()

    plt.tight_layout()
    plt.show()
