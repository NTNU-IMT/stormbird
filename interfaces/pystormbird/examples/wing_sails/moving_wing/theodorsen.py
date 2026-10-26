'''
Theodorsen's model for the unsteady lift on a two-dimensional foil in harmonic motion.

Only the circulatory lift is included, i.e., the lift due to the circulation around the foil, which
is affected by the vorticity shed into the wake. The added mass (non-circulatory) lift is not
included.

The model is described in, for instance:
Theodorsen, T., 1935, "General theory of aerodynamic instability and the mechanism of flutter",
NACA Report No. 496.
'''

import numpy as np
from scipy.special import hankel2

def theodorsen_function(reduced_frequency: np.ndarray | float) -> np.ndarray:
    '''
    Theodorsen's function, C(k) = F(k) + i G(k), as a function of the reduced frequency,
    k = omega * b / U, where b is half the chord length.

    The function is computed from the Hankel functions of the second kind:

        C(k) = H1(k) / (H1(k) + i H0(k))

    The magnitude, |C(k)|, is the reduction of the amplitude of the circulatory lift compared to the
    quasi-steady lift, which goes from 1.0 at k = 0 to 0.5 for large k. The phase, arg C(k), is
    negative, meaning that the circulatory lift lags behind the quasi-steady lift.
    '''
    k = np.asarray(reduced_frequency, dtype=float)

    # The Hankel functions are singular at zero, where the limit is C = 1
    k_safe = np.where(k > 0.0, k, 1.0)

    h0 = hankel2(0, k_safe)
    h1 = hankel2(1, k_safe)

    return np.where(k > 0.0, h1 / (h1 + 1j * h0), 1.0 + 0.0j)

def heave_circulatory_lift_coefficient(
    time: np.ndarray,
    *,
    amplitude: float,
    radial_frequency: float,
    velocity: float,
    chord_length: float,
    lift_slope: float = 2.0 * np.pi
) -> np.ndarray:
    '''
    Circulatory lift coefficient for a foil in harmonic heave motion, h(t) = amplitude * sin(w t),
    in the same direction as the lift.

    The heave velocity gives an effective angle of attack, alpha(t) = -dh/dt / U, which gives the
    quasi-steady lift coefficient lift_slope * alpha(t). In Theodorsen's model, the circulatory
    lift is the quasi-steady lift multiplied with the Theodorsen function. For harmonic motion, this
    means that the amplitude is multiplied with |C(k)| and the phase is shifted with arg C(k):

        C_L(t) = -lift_slope * (amplitude * w / U) * |C(k)| * cos(w t + arg C(k))

    The result is the periodic solution, after any start-up effects have died out.

    Args:
        time: the time values to compute the lift coefficient at
        amplitude: the amplitude of the heave motion
        radial_frequency: the radial frequency of the heave motion
        velocity: the freestream velocity
        chord_length: the chord length of the foil
        lift_slope: the quasi-steady lift slope. The default value is 2 pi, which is the
            two-dimensional value. A different value can be used to approximate three-dimensional
            effects.
    '''
    reduced_frequency = radial_frequency * 0.5 * chord_length / velocity

    theodorsen = theodorsen_function(reduced_frequency)

    quasi_steady_amplitude = lift_slope * amplitude * radial_frequency / velocity

    return -quasi_steady_amplitude * np.abs(theodorsen) * np.cos(
        radial_frequency * np.asarray(time) + np.angle(theodorsen)
    )
