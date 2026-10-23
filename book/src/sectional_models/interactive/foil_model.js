// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// JavaScript copy of the `Foil` model in `stormbird/src/section_models/foil.rs`, used for the
// interactive plot in the book. The functions mirror the Rust implementation one-to-one, and must
// be updated whenever the Rust model changes.

(function (root) {
    'use strict';

    const TAU = 2.0 * Math.PI;

    // Same slope constant as in `stormath::transition_functions::sigmoid_zero_to_one`
    const SIGMOID_SLOPE = 4.5951212;

    const toRadians = (degrees) => degrees * Math.PI / 180.0;

    function sigmoidZeroToOne(x, x0, transitionRange) {
        const slope = SIGMOID_SLOPE / transitionRange;

        return 1.0 / (1.0 + Math.exp(-slope * (x - x0)));
    }

    function integratedSigmoidFromZero(x, x0, transitionRange) {
        if (transitionRange <= 0.0) {
            return Math.max(x - x0, 0.0) - Math.max(-x0, 0.0);
        }

        const slope = SIGMOID_SLOPE / transitionRange;

        const softplus = (z) => Math.max(z, 0.0) + Math.log1p(Math.exp(-Math.abs(z)));

        return (softplus(slope * (x - x0)) - softplus(-slope * x0)) / slope;
    }

    const DEFAULT_PARAMETERS = Object.freeze({
        cl_zero_angle: 0.0,
        cl_initial_slope: TAU,
        cl_slope_reduction_factor_positive: 0.0,
        cl_slope_reduction_factor_negative: 0.0,
        cl_slope_reduction_onset_angle_positive: toRadians(10.0),
        cl_slope_reduction_onset_angle_negative: toRadians(10.0),
        cl_slope_reduction_range: toRadians(6.0),
        cl_max_after_stall: 1.0,
        cd_min: 0.0,
        cd_second_order_factor: 0.0,
        cd_max_after_stall: 1.0,
        cd_power_after_stall: 1.6,
        mean_positive_stall_angle: toRadians(20.0),
        mean_negative_stall_angle: toRadians(20.0),
        stall_range: toRadians(6.0),
    });

    class Foil {
        constructor(parameters) {
            Object.assign(this, DEFAULT_PARAMETERS, parameters);
        }

        liftCoefficient(angleOfAttack) {
            return this.liftCoefficientPreStallWithStallDropOff(angleOfAttack) +
                this.liftCoefficientPostStallWithStallWeight(angleOfAttack);
        }

        liftCoefficientLinear(angleOfAttack) {
            return this.cl_zero_angle + this.cl_initial_slope * Math.sin(angleOfAttack);
        }

        liftCoefficientPreStallRaw(angleOfAttack) {
            const x = Math.sin(angleOfAttack);

            const xOnsetPositive = Math.sin(Math.abs(this.cl_slope_reduction_onset_angle_positive));
            const xOnsetNegative = Math.sin(Math.abs(this.cl_slope_reduction_onset_angle_negative));

            const reductionPositive = integratedSigmoidFromZero(
                x, xOnsetPositive, this.cl_slope_reduction_range
            );

            const reductionNegative = integratedSigmoidFromZero(
                -x, xOnsetNegative, this.cl_slope_reduction_range
            );

            return this.liftCoefficientLinear(angleOfAttack) + this.cl_initial_slope * (
                -this.cl_slope_reduction_factor_positive * reductionPositive +
                this.cl_slope_reduction_factor_negative * reductionNegative
            );
        }

        liftCoefficientPreStallWithStallDropOff(angleOfAttack) {
            return this.liftCoefficientPreStallRaw(angleOfAttack) *
                (1.0 - this.amountOfStall(angleOfAttack));
        }

        liftCoefficientPostStallRaw(angleOfAttack) {

            return this.cl_max_after_stall * Math.sin(2.0 * angleOfAttack);
        }

        liftCoefficientPostStallWithStallWeight(angleOfAttack) {
            return this.liftCoefficientPostStallRaw(angleOfAttack) *
                this.amountOfStall(angleOfAttack);
        }

        dragCoefficientPreStall(angleOfAttack) {
            const effectiveAngle = this.cl_initial_slope !== 0.0 ?
                this.liftCoefficientLinear(angleOfAttack) / this.cl_initial_slope :
                Math.sin(angleOfAttack);

            return this.cd_min + this.cd_second_order_factor * effectiveAngle ** 2;
        }

        dragCoefficientPostStallRaw(angleOfAttack) {
            return this.cd_max_after_stall *
                Math.abs(Math.sin(angleOfAttack)) ** this.cd_power_after_stall;
        }

        dragCoefficient(angleOfAttack) {
            const cdPreStall = this.dragCoefficientPreStall(angleOfAttack);
            const cdPostStall = this.dragCoefficientPostStallRaw(angleOfAttack);

            const amountOfStall = this.amountOfStall(angleOfAttack);

            return cdPreStall * (1.0 - amountOfStall) + amountOfStall * cdPostStall;
        }

        amountOfStall(angleOfAttack) {
            const meanStallAngle = angleOfAttack >= 0.0 ?
                Math.abs(this.mean_positive_stall_angle) :
                Math.abs(this.mean_negative_stall_angle);

            return sigmoidZeroToOne(Math.abs(angleOfAttack), meanStallAngle, this.stall_range);
        }
    }

    const api = { Foil, DEFAULT_PARAMETERS };

    if (typeof module !== 'undefined' && module.exports) {
        module.exports = api;
    } else {
        root.StormbirdFoilModel = api;
    }
})(typeof window !== 'undefined' ? window : globalThis);
