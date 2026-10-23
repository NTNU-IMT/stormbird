// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// JavaScript copy of the `RotatingCylinder` model in
// `stormbird/src/section_models/rotating_cylinder.rs`, used for the interactive plot in the book.
// The functions mirror the Rust implementation one-to-one, and must be updated whenever the Rust
// model changes. The power coefficient is not included, as it is not plotted.

(function (root) {
    'use strict';

    // Same slope constant as in `stormath::transition_functions::sigmoid_zero_to_one`
    const SIGMOID_SLOPE = 4.5951212;

    const toRadians = (degrees) => degrees * Math.PI / 180.0;

    // Same as `integrated_sigmoid_from_zero` in `stormbird/src/section_models/mod.rs`
    function integratedSigmoidFromZero(x, x0, transitionRange) {
        if (transitionRange <= 0.0) {
            return Math.max(x - x0, 0.0) - Math.max(-x0, 0.0);
        }

        const slope = SIGMOID_SLOPE / transitionRange;

        const softplus = (z) => Math.max(z, 0.0) + Math.log1p(Math.exp(-Math.abs(z)));

        return (softplus(slope * (x - x0)) - softplus(-slope * x0)) / slope;
    }

    const DEFAULT_PARAMETERS = Object.freeze({
        cl_initial_slope: 3.0,
        cl_high_spin_ratio_slope: 1.5,
        cl_slope_transition_spin_ratio: 3.4,
        cl_slope_transition_range: 1.0,
        cd_start: 0.457,
        cd_end: 0.05,
        cd_angle_start: toRadians(5.0),
        cd_spin_ratio_low_drag: 2.7,
    });

    class RotatingCylinder {
        constructor(parameters) {
            Object.assign(this, DEFAULT_PARAMETERS, parameters);
        }

        liftCoefficient(spinRatio) {
            const transitionSpinRatio = Math.abs(this.cl_slope_transition_spin_ratio);

            const transition = integratedSigmoidFromZero(
                spinRatio, transitionSpinRatio, this.cl_slope_transition_range
            ) - integratedSigmoidFromZero(
                -spinRatio, transitionSpinRatio, this.cl_slope_transition_range
            );

            return this.cl_initial_slope * spinRatio -
                (this.cl_initial_slope - this.cl_high_spin_ratio_slope) * transition;
        }

        dragCoefficient(spinRatio) {
            const angle0 = Math.PI * Math.abs(spinRatio) / this.cd_spin_ratio_low_drag;
            const angle = angle0 + this.cd_angle_start;

            const angleEffective = Math.PI * angle / (Math.PI + this.cd_angle_start);

            if (angleEffective < Math.PI) {
                const c1 = (1.0 + Math.cos(angleEffective)) / 2.0;
                const c0 = (1.0 + Math.cos(this.cd_angle_start)) / 2.0;

                const s = c1 / c0;

                return this.cd_start * s + (1.0 - s) * this.cd_end;
            }

            return this.cd_end;
        }
    }

    const api = { RotatingCylinder, DEFAULT_PARAMETERS };

    if (typeof module !== 'undefined' && module.exports) {
        module.exports = api;
    } else {
        root.StormbirdRotatingCylinderModel = api;
    }
})(typeof window !== 'undefined' ? window : globalThis);
