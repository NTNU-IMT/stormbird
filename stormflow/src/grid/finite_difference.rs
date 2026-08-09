use stormath::type_aliases::Float;

/// Highest derivative order [`weights`] can be asked for.
const MAX_DERIVATIVE_ORDER: usize = 2;

/// Finite-difference (and interpolation) weights on arbitrarily spaced nodes, via Fornberg's
/// algorithm.
///
/// Returns the weights `w` such that
///
/// ```text
/// f^(derivative_order)(evaluation_point) ~= sum_j w[j] * f(nodes[j])
/// ```
///
/// The result is exact for every polynomial of degree up to `N - 1`, so an `N`-node stencil for the
/// first derivative is `N - 1`-order accurate (one order less when the stencil is symmetric about
/// the evaluation point and the leading error term cancels). `derivative_order == 0` gives plain
/// Lagrange interpolation weights.
///
/// `evaluation_point` does not have to be one of the nodes, which is what makes this usable for the
/// staggered grid: cell-centered samples are differentiated onto faces and vice versa.
///
/// On equidistant nodes this reproduces the classical textbook coefficients exactly (the weights
/// solving the polynomial-exactness conditions are unique), which is what keeps the solver's
/// behaviour unchanged on a uniform grid.
pub fn weights<const N: usize>(
    derivative_order: usize,
    evaluation_point: Float,
    nodes: [Float; N],
) -> [Float; N] {
    assert!(
        derivative_order <= MAX_DERIVATIVE_ORDER,
        "finite_difference::weights supports derivative orders up to {MAX_DERIVATIVE_ORDER}, got {derivative_order}"
    );
    assert!(
        N > derivative_order,
        "a stencil for derivative order {derivative_order} needs more than {derivative_order} nodes, got {N}"
    );

    let m = derivative_order;

    // `coefficients[j][k]` holds the weight of node `j` for the `k`-th derivative, built up one
    // node at a time. Only column `m` is returned; the lower columns are the intermediate values
    // the recurrence needs.
    let mut coefficients = [[0.0 as Float; MAX_DERIVATIVE_ORDER + 1]; N];
    coefficients[0][0] = 1.0;

    let mut c1 = 1.0;
    let mut c4 = nodes[0] - evaluation_point;

    for i in 1..N {
        let highest_order_so_far = m.min(i);

        let mut c2 = 1.0;
        let c5 = c4;

        c4 = nodes[i] - evaluation_point;

        for j in 0..i {
            let c3 = nodes[i] - nodes[j];

            debug_assert!(c3 != 0.0, "finite difference nodes must be distinct");

            c2 *= c3;

            if j == i - 1 {
                for k in (1..=highest_order_so_far).rev() {
                    coefficients[i][k] = c1 * (
                        (k as Float) * coefficients[i - 1][k - 1] - c5 * coefficients[i - 1][k]
                    ) / c2;
                }

                coefficients[i][0] = -c1 * c5 * coefficients[i - 1][0] / c2;
            }

            for k in (1..=highest_order_so_far).rev() {
                coefficients[j][k] = (
                    c4 * coefficients[j][k] - (k as Float) * coefficients[j][k - 1]
                ) / c3;
            }

            coefficients[j][0] = c4 * coefficients[j][0] / c3;
        }

        c1 = c2;
    }

    let mut out: [Float; N] = std::array::from_fn(|j| coefficients[j][m]);

    enforce_consistency(&mut out, m);

    out
}

/// Removes the round-off in the weights' sum.
///
/// Exactness for the constant function `f = 1` requires the weights to sum to `1` for an
/// interpolation and to `0` for any derivative. Fornberg's recurrence only satisfies that up to
/// floating point round-off, and in single precision the leftover matters: the hardcoded kernels
/// this replaced were written as differences (`27 * (u_p - u_0) - (u_p2 - u_n)`), so a constant
/// field cancelled to *exactly* zero divergence. Contracting with weights instead leaves
/// `(sum of weights) * u`, which for a 10 m/s freestream showed up as a spurious ~1e-5 divergence
/// everywhere for the pressure solve to chase.
///
/// The correction is put on the largest-magnitude weight, which keeps the relative perturbation as
/// small as possible and leaves the stencil's accuracy untouched.
fn enforce_consistency<const N: usize>(weights: &mut [Float; N], derivative_order: usize) {
    let target_sum = if derivative_order == 0 { 1.0 } else { 0.0 };

    let mut sum = 0.0 as Float;
    let mut largest = 0usize;

    for j in 0..N {
        sum += weights[j];

        if weights[j].abs() > weights[largest].abs() {
            largest = j;
        }
    }

    weights[largest] += target_sum - sum;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close<const N: usize>(actual: [Float; N], expected: [Float; N], tolerance: Float) {
        for j in 0..N {
            assert!(
                (actual[j] - expected[j]).abs() < tolerance,
                "weight {j}: got {}, expected {}", actual[j], expected[j]
            );
        }
    }

    /// The uniform-grid stencils the solver used before the grid was generalized. Each of these is
    /// the unique set of weights that is exact for polynomials up to degree `N - 1`, so the
    /// generalized routine has to reproduce them exactly on equidistant nodes.
    #[test]
    fn reproduces_the_classical_uniform_coefficients() {
        let h = 0.25 as Float;

        // `interp4`: cell-centered samples at -1.5h..1.5h interpolated onto the face at 0.
        assert_close(
            weights(0, 0.0, [-1.5 * h, -0.5 * h, 0.5 * h, 1.5 * h]),
            [-1.0 / 16.0, 9.0 / 16.0, 9.0 / 16.0, -1.0 / 16.0],
            1e-6
        );

        // The staggered 4-point first derivative used by the pressure gradient and the divergence.
        assert_close(
            weights(1, 0.0, [-1.5 * h, -0.5 * h, 0.5 * h, 1.5 * h]),
            [1.0 / (24.0 * h), -27.0 / (24.0 * h), 27.0 / (24.0 * h), -1.0 / (24.0 * h)],
            1e-4
        );

        // `laplacian4`: [-1, 16, -30, 16, -1] / (12 h^2)
        assert_close(
            weights(2, 0.0, [-2.0 * h, -h, 0.0, h, 2.0 * h]),
            [
                -1.0 / (12.0 * h * h),
                16.0 / (12.0 * h * h),
                -30.0 / (12.0 * h * h),
                16.0 / (12.0 * h * h),
                -1.0 / (12.0 * h * h),
            ],
            1e-2
        );

        // `upwind_derivative4_plus`: [-1, 6, -18, 10, 3] / (12 h)
        assert_close(
            weights(1, 0.0, [-3.0 * h, -2.0 * h, -h, 0.0, h]),
            [
                -1.0 / (12.0 * h),
                6.0 / (12.0 * h),
                -18.0 / (12.0 * h),
                10.0 / (12.0 * h),
                3.0 / (12.0 * h),
            ],
            1e-3
        );

        // `upwind_derivative4_minus`: [-3, -10, 18, -6, 1] / (12 h)
        assert_close(
            weights(1, 0.0, [-h, 0.0, h, 2.0 * h, 3.0 * h]),
            [
                -3.0 / (12.0 * h),
                -10.0 / (12.0 * h),
                18.0 / (12.0 * h),
                -6.0 / (12.0 * h),
                1.0 / (12.0 * h),
            ],
            1e-3
        );

        // Plain 3-point second derivative.
        assert_close(
            weights(2, 0.0, [-h, 0.0, h]),
            [1.0 / (h * h), -2.0 / (h * h), 1.0 / (h * h)],
            1e-2
        );
    }

    /// On non-uniform nodes the defining property is polynomial exactness, so check it directly
    /// rather than against a closed form.
    #[test]
    fn is_exact_for_polynomials_on_non_uniform_nodes() {
        let nodes = [-0.7 as Float, -0.25, 0.1, 0.6, 1.9];
        let evaluation_point = 0.05 as Float;

        // f(x) = 2 + 3x - 1.5x^2 + 0.4x^3 - 0.2x^4
        let f = |x: Float| 2.0 + 3.0 * x - 1.5 * x * x + 0.4 * x.powi(3) - 0.2 * x.powi(4);
        let df = |x: Float| 3.0 - 3.0 * x + 1.2 * x * x - 0.8 * x.powi(3);
        let ddf = |x: Float| -3.0 + 2.4 * x - 2.4 * x * x;

        let sampled: Vec<Float> = nodes.iter().map(|&x| f(x)).collect();

        for (order, expected) in [
            (0usize, f(evaluation_point)),
            (1, df(evaluation_point)),
            (2, ddf(evaluation_point)),
        ] {
            let w = weights(order, evaluation_point, nodes);
            let approximation: Float = (0..5).map(|j| w[j] * sampled[j]).sum();

            assert!(
                (approximation - expected).abs() < 1e-4,
                "order {order}: got {approximation}, expected {expected}"
            );
        }
    }
}
