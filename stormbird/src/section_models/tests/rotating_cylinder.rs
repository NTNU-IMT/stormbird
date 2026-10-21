use stormath::type_aliases::Float;

use crate::section_models::rotating_cylinder::RotatingCylinder;

#[test]
fn rotating_cylinder_power_coefficient_follows_tillig_polynomial() {
    let cylinder = RotatingCylinder::default();

    for spin_ratio in [0.0, 1.0, 2.5, 4.0, -3.0] {
        let sr: Float = Float::abs(spin_ratio);

        let expected = 0.0001 * sr.powi(5) - 0.0004 * sr.powi(4) + 0.0143 * sr.powi(3)
            - 0.0168 * sr.powi(2) + 0.0234 * sr;

        assert!((cylinder.power_coefficient(spin_ratio) - expected).abs() < 1e-12);
    }
}
