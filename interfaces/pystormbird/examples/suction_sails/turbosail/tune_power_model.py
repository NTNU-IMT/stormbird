'''
Tunes the internal flow model of the suction sail to match the power curve from the experiments.

The power curve gives the lift coefficient as a function of the power coefficient, but not the
angle of attack or the suction coefficient for each point. It is therefore assumed that the sail is
operated at the maximum lift for each suction coefficient. As the power in the model only depends
on the suction coefficient, and increases with it, this is the operating point that gives the
lowest power for a given lift.

The power on each line element follows from the internal flow model. With a given suction
coefficient, Cq, the internal suction coefficient, sigma = s / (0.5 rho U^2), is:

    sigma = (K / phi^2) Cq^2 - cp_ext,    cp_ext = cp_no_suction + cp_suction_gradient Cq

where K is the loss coefficient, phi is the porous fraction and cp_ext is the external pressure
coefficient. The power for the whole wing is then:

    P = sum_i s_i Q_i / eta = sum_i 0.5 rho A_i U_i^3 Cq_i sigma_i / eta

This is linear in K, cp_no_suction and cp_suction_gradient, which are therefore found with linear
least squares, using simulations with a constant suction coefficient along the span. The tuned
model is then verified with simulations where the power is the internal state.
'''

import numpy as np
import matplotlib.pyplot as plt

from stormbird_setup.section_models import InternalFlow, InternalStateConversion

from suction_sail_data import CQ_VALUES_3D, power_curve_data
from suction_sail_model import SuctionSailModel, SimulationResult, SUCTION_COEFFICIENT_DIRECT

# Fixed parameters. Only the ratio porous_fraction / sqrt(loss_coefficient) affects the flow, so the
# porous fraction is set from the geometry and the loss coefficient is tuned. The fan efficiency
# scales all the terms in the power equally, and can not be found from the power curve.
POROUS_FRACTION = 0.25
FAN_EFFICIENCY = 0.7

PARAMETER_NAMES = ["loss_coefficient", "cp_no_suction", "cp_suction_gradient"]

# The parameters that are kept fixed, with their values, while the others are tuned. The three
# parameters can not all be tuned, as the effect of the gradient on the power is close to a
# combination of the effects of the other two. Here, the external pressure is assumed to be zero
# without suction, so that it is only the suction that lowers it.
FIXED_PARAMETERS = {"cp_no_suction": 0.0}

# Other choices of fixed parameters, which are compared to show the effect of the choice
COMPARED_FIXED_PARAMETERS = [
    {"cp_no_suction": 0.0},
    {"cp_no_suction": -0.5},
    {"cp_no_suction": -1.0},
    {"cp_suction_gradient": 0.0},
    {"cp_suction_gradient": -20.0},
    {},
]

VELOCITY = 10.0

# Coarse sweep for the maximum lift, which occurs between 15 and 40 degrees for all suction
# coefficients in the model. The angle is refined around the best value.
ANGLES_OF_ATTACK_DEG = np.arange(10.0, 49.0, 2.0)


def power_coefficient_terms(result: SimulationResult) -> np.ndarray:
    '''
    Returns the terms that are multiplied with the loss coefficient, cp_no_suction and
    cp_suction_gradient to give the power coefficient for the wing, for a simulation with a given
    suction coefficient on each line element.
    '''
    real = ~result.section_is_virtual

    cq = np.abs(result.section_internal_states[real])
    u = result.section_velocities[real] / VELOCITY

    # All line elements have the same length
    area_fraction = 1.0 / len(cq)

    weight = area_fraction * u**3 / FAN_EFFICIENCY

    return np.array([
        np.sum(weight * cq**3) / POROUS_FRACTION**2,
        -np.sum(weight * cq),
        -np.sum(weight * cq**2),
    ])


def fit_parameters(
    terms: np.ndarray,
    cp_target: np.ndarray,
    error_weight: np.ndarray,
    fixed_parameters: dict[str, float]
) -> tuple[dict[str, float], float]:
    '''
    Finds the parameters that are not fixed with linear least squares, where the error in the power
    coefficient is multiplied with the given weight. Returns all the parameters, and the condition
    number of the scaled problem, which shows how well the tuned parameters are determined by the
    data.
    '''
    tuned = [i for i, name in enumerate(PARAMETER_NAMES) if name not in fixed_parameters]
    fixed = [i for i, name in enumerate(PARAMETER_NAMES) if name in fixed_parameters]

    fixed_values = np.array([fixed_parameters[PARAMETER_NAMES[i]] for i in fixed])

    matrix = error_weight[:, None] * terms[:, tuned]
    rhs = error_weight * (cp_target - terms[:, fixed] @ fixed_values)

    # The columns are normalized, so that the condition number does not depend on the units
    column_norms = np.linalg.norm(matrix, axis=0)

    solution, _, _, singular_values = np.linalg.lstsq(matrix / column_norms, rhs)

    parameters = dict(fixed_parameters)
    for i, value in zip(tuned, solution / column_norms):
        parameters[PARAMETER_NAMES[i]] = float(value)

    return parameters, singular_values[0] / singular_values[-1]


def parameter_vector(parameters: dict[str, float]) -> np.ndarray:
    return np.array([parameters[name] for name in PARAMETER_NAMES])


if __name__ == "__main__":
    cp_data, cl_data = power_curve_data()

    # Simulations with constant suction coefficient, at the maximum lift
    cq_values = np.concatenate([
        np.linspace(0.005, 0.015, 3),
        np.linspace(0.016, 0.02, 5),
        np.linspace(0.023, CQ_VALUES_3D[-1], 10),
    ])

    direct_model = SuctionSailModel(internal_state_conversion = SUCTION_COEFFICIENT_DIRECT)

    cl_max = np.zeros(len(cq_values))
    terms = np.zeros((len(cq_values), len(PARAMETER_NAMES)))

    for i, cq in enumerate(cq_values):
        result = direct_model.max_lift_case(
            velocity = VELOCITY,
            angles_of_attack_deg = ANGLES_OF_ATTACK_DEG,
            internal_state = cq
        )

        cl_max[i] = result.cl
        terms[i] = power_coefficient_terms(result)

        print(f"Cq = {cq:.4f}: CL max = {cl_max[i]:.3f}")

    # Power coefficient and slope of the experimental power curve at the same lift, for the points
    # within the range of the experimental data
    in_range = (cl_max >= cl_data[0]) & (cl_max <= cl_data[-1])

    cp_target = np.interp(cl_max[in_range], cl_data, cp_data)
    cl_slope = np.interp(cl_max[in_range], cl_data, np.gradient(cl_data, cp_data))

    # The error in the power coefficient is converted to the distance to the experimental curve,
    # normal to the curve, in coordinates scaled with the range of the data. On the flat part of
    # the curve, this is close to the error in the lift, and on the steep part, close to the error
    # in the power. Otherwise, the points where a small difference in one of them gives a large
    # difference in the other would dominate the fit.
    cp_scale = cp_data[-1] - cp_data[0]
    cl_scale = cl_data[-1] - cl_data[0]

    scaled_slope = cl_slope * cp_scale / cl_scale

    error_weight = scaled_slope / np.sqrt(1.0 + scaled_slope**2) / cp_scale

    # Comparison of different choices of fixed parameters
    print()
    for fixed_parameters in COMPARED_FIXED_PARAMETERS:
        parameters, condition_number = fit_parameters(
            terms[in_range], cp_target, error_weight, fixed_parameters
        )

        distance = error_weight * (terms[in_range] @ parameter_vector(parameters) - cp_target)

        label = ", ".join(fixed_parameters) + " fixed" if fixed_parameters else "all tuned"

        print(
            f"{label:30s}: " +
            ", ".join(f"{name} = {parameters[name]:8.3f}" for name in PARAMETER_NAMES) +
            f", condition number = {condition_number:6.1f}" +
            f", RMS scaled distance = {np.sqrt(np.mean(distance**2)):.4f}"
        )

    parameters, _ = fit_parameters(terms[in_range], cp_target, error_weight, FIXED_PARAMETERS)

    cp_fit = terms @ parameter_vector(parameters)

    print()
    print(f"Used: {parameters}")

    # Verification, with the power as the internal state
    internal_flow = InternalFlow(
        cp_no_suction = parameters["cp_no_suction"],
        cp_suction_gradient = parameters["cp_suction_gradient"],
        loss_coefficient = parameters["loss_coefficient"],
        porous_fraction = POROUS_FRACTION,
    )

    power_model = SuctionSailModel(
        internal_state_conversion = InternalStateConversion.new_power_to_internal_flow(
            internal_flow = internal_flow,
            fan_efficiency = FAN_EFFICIENCY
        )
    )

    power_factor = 0.5 * power_model.density * power_model.area * VELOCITY**3

    cp_verification = np.geomspace(0.5 * cp_data[0], cp_data[-1], 14)
    cl_verification = np.zeros(len(cp_verification))

    for i, cp in enumerate(cp_verification):
        result = power_model.max_lift_case(
            velocity = VELOCITY,
            angles_of_attack_deg = ANGLES_OF_ATTACK_DEG,
            internal_state = cp * power_factor
        )

        cl_verification[i] = result.cl

        print(
            f"Cp = {cp:.4f}: CL max = {result.cl:.3f}, "
            f"wing Cq = {result.wing_suction_coefficient(VELOCITY):.4f}"
        )

    fig = plt.figure(figsize = (8, 5.5))
    ax = fig.add_subplot(111)

    ax.plot(cp_data, cl_data, "o--", color = "black", label = "Experiments")
    ax.plot(
        cp_fit, cl_max, "s",
        markerfacecolor = "none",
        label = "Model, constant Cq along span"
    )
    ax.plot(cp_verification, cl_verification, "-", label = "Model, power as internal state")

    ax.set_xlabel("Power coefficient (CP)")
    ax.set_ylabel("Maximum lift coefficient (CL)")
    ax.set_title(
        f"K = {parameters['loss_coefficient']:.2f}, "
        f"cp_no_suction = {parameters['cp_no_suction']:.2f}, "
        f"cp_suction_gradient = {parameters['cp_suction_gradient']:.2f}, "
        f"porous fraction = {POROUS_FRACTION}, fan efficiency = {FAN_EFFICIENCY}",
        fontsize = 9
    )

    ax.legend()

    plt.tight_layout()
    plt.savefig("power_curve_match.png", dpi = 150)
    plt.show()
