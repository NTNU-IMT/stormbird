from ...base_model import StormbirdSetupBaseModel

import numpy as np
from scipy.optimize import curve_fit

DEFAULT_VON_KARMAN_CONSTANT = 0.41
DEFAULT_REFERENCE_HEIGHT = 10.0

DEFAULT_MIN_SURFACE_ROUGHNESS = 0.00001
DEFAULT_MAX_SURFACE_ROUGHNESS = 0.1

DELTA_VELOCITY_RATIO_LIMIT = 0.01

class LogarithmicModel(StormbirdSetupBaseModel):
    """
    Logarithmic model of how velocity varies as a function of height, which also includes optional
    stability corrections.
    """

    friction_velocity: float
    """
    The frictional velocity, defined as the square root of the surface friction from the wind on 
    the ground or ocean divided by density. This type of parameter is often available directly 
    in hindcast data.
    """

    surface_roughness: float
    """
    The surface roughness of the ground or ocean (e.g., how much waves). Can either be computed 
    based if some reference velocity and the frictional velocity is known, or set directly from
    hindcast data.
    """

    obukhov_length: float | None = None
    """
    The Obukhov length (https://en.wikipedia.org/wiki/Monin-Obukhov_length) is used to 
    correct the profile for non-neutral conditions, e.g., stable or unstable atmospheres.
    """

    von_karman_constant: float = DEFAULT_VON_KARMAN_CONSTANT
    """The Von Karman constant (https://en.wikipedia.org/wiki/Von_Kármán_constant)."""

    stable_coefficient: float = 6.0
    """
    Coefficient used in the model for a stable stability correction. It is typically fixed for a
    given case (e.g., it is not supposed to depend on the weather), so the default value 
    should be useful most of the time. However, it is allowed to vary, because different papers
    use slight variations here.
    """

    unstable_coefficient: float = 19.3
    """
    Coefficient used in the model for an unstable stability correction. As for the stable 
    coefficient above, it is typically fixed, but different papers use slight variations.
    """

    @classmethod
    def new_neutral(
        cls,
        *,
        velocity: float,
        friction_velocity: float,
        reference_height: float = DEFAULT_REFERENCE_HEIGHT,
        von_karman_constant: float = DEFAULT_VON_KARMAN_CONSTANT,
        min_surface_roughness: float = DEFAULT_MIN_SURFACE_ROUGHNESS,
        max_surface_roughness: float = DEFAULT_MAX_SURFACE_ROUGHNESS
    ) -> "LogarithmicModel":
        # Start by calculating the roughness based on the raw data
        exp_factor = velocity * von_karman_constant / friction_velocity
        
        surface_roughness = reference_height / np.exp(exp_factor)

        # Check that it does noe violate the supplied limits
        if surface_roughness < min_surface_roughness:
            surface_roughness = min_surface_roughness
        if surface_roughness > max_surface_roughness:
            surface_roughness = max_surface_roughness

        # Recalculate the friction velocity, based on the potentially limited roughness
        friction_velocity = (
            velocity * von_karman_constant / 
            np.log(reference_height / surface_roughness)
        )

        return cls(
            friction_velocity = friction_velocity,
            surface_roughness = surface_roughness,
            von_karman_constant = von_karman_constant
        )

    @classmethod
    def new_with_stability_correction(
        cls,
        *,
        velocity: float,
        velocity_neutral: float,
        friction_velocity: float,
        reference_height: float = DEFAULT_REFERENCE_HEIGHT,
        von_karman_constant: float = DEFAULT_VON_KARMAN_CONSTANT
    ) -> "LogarithmicModel":
        delta_velocity = velocity_neutral - velocity

        delta_velocity_ratio = np.abs(delta_velocity / velocity)

        if delta_velocity_ratio <= DELTA_VELOCITY_RATIO_LIMIT:
            return cls.new_neutral(
                velocity = velocity,
                friction_velocity = friction_velocity,
                reference_height = reference_height,
                von_karman_constant = von_karman_constant
            )

        out = cls.new_neutral(
            velocity = velocity_neutral,
            friction_velocity = friction_velocity,
            reference_height = reference_height,
            von_karman_constant = von_karman_constant
        )

        delta_velocity = velocity_neutral - velocity
        
        if delta_velocity > 0.0:
            stable = False
        else:
            stable = True

        if stable:
            out.fit_obukhov_length_stable(
                velocity_neutral=velocity_neutral,
                velocity=velocity,
                reference_height=reference_height
            )
        else:
            height_values = [reference_height]
            velocity_values = [velocity]
            
            out.fit_obukhov_length_unstable(height_values, velocity_values)

        return out

    def fit_obukhov_length_stable(
        self,
        *,
        velocity_neutral: float,
        velocity: float,
        reference_height: float = DEFAULT_REFERENCE_HEIGHT
    ):
        delta_velocity = velocity_neutral - velocity
        psi_target = self.von_karman_constant * delta_velocity / self.friction_velocity
        
        zeta = -psi_target / self.stable_coefficient
        self.obukhov_length = reference_height / zeta

    def fit_obukhov_length_unstable(
        self,
        height_values: list[float], 
        velocity_values: list[float]
    ):
        def true_wind_velocity_at_heights(heights: np.ndarray) -> np.ndarray:
            out = np.zeros(len(heights))
            
            for i in range(len(heights)):
                out[i] = self.velocity_at_height(heights[i])
                
            return out
        
        def obj_func(height: np.ndarray, length: float) -> np.ndarray:
            self.obukhov_length = length
            
            return true_wind_velocity_at_heights(height)
            
        bounds = (-2000.0, -1.0)
            
        popt, _pcov = curve_fit(
            f = obj_func, 
            xdata = height_values, 
            ydata = velocity_values, 
            bounds = bounds
        )
        
        self.obukhov_length = popt[0]
        
    
    def velocity_at_height(self, height: float) -> float:
        return self.neutral_velocity_at_height(height) - self.businger_dyer_correction(height)

    def scale_factor(self) -> float:
        return self.friction_velocity / self.von_karman_constant

    def neutral_velocity_at_height(self, height: float) -> float:
        undefined_condition = self.surface_roughness <= 0.0 or height <= 0.0

        if undefined_condition:
            return 0.0

        return self.scale_factor() * np.log(height / self.surface_roughness)

    def businger_dyer_unscaled_correction(self, height: float) -> float:
        """
        The equations that compute the Businger Dyer correction to account for non-neutral 
        atmosphere.
        """
        if self.obukhov_length is None:
            return 0.0

        length = self.obukhov_length
        zeta = height / length

        if length > 0.0:
            # Stable
            return -self.stable_coefficient * zeta
        else:
            # Unstable
            x = (1.0 - self.unstable_coefficient * zeta) ** 0.25

            first_term = 2.0 * np.log((1.0 + x) / 2.0)
            second_term = np.log((1.0 + x**2) / 2.0)
            third_term = -2.0 * np.arctan(x) + np.pi / 2.0

            return first_term + second_term + third_term

    def businger_dyer_correction(self, height: float) -> float:
        """
        Function that computes the Businger-Dyer correction based on the atmosphere state and 
        the Obukhov length.
        """
        non_scaled_correction = self.businger_dyer_unscaled_correction(height)

        return non_scaled_correction * self.scale_factor()
