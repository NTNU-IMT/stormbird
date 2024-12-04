import numpy as np
from pymathutils import smoothing
import matplotlib.pyplot as plt


if __name__ == '__main__':
    x = np.linspace(-0.5, 0.5, 100)
    y = np.sqrt(1 - (x * 2) ** 2)

    y_with_noise = y + np.random.normal(0, 0.1, 100)

    end_conditions = ["ZeroValues", "ReversedMirroredValues"]

    y_gaussian = smoothing.gaussian_smoothing(
        x=x, 
        y=y_with_noise, 
        smoothing_length= 0.05,
        number_of_end_insertions=20,
        end_conditions = end_conditions
    )

    y_poly_five = smoothing.cubic_polynomial_smoothing(
        y = y_with_noise,
        end_conditions = end_conditions,
        window_size = "Five"
    )

    y_poly_nine = smoothing.cubic_polynomial_smoothing(
        y = y_with_noise,
        end_conditions = end_conditions,
        window_size = "Nine"
    )

    plt.plot(x, y, label='Original')
    plt.plot(x, y_with_noise, label='With noise')
    plt.plot(x, y_gaussian, label='Gaussian smoothing')
    plt.plot(x, y_poly_five, label='Cubic polynomial smoothing, window size 5')
    plt.plot(x, y_poly_nine, label='Cubic polynomial smoothing, window size 9')

    plt.legend()
    plt.show()
