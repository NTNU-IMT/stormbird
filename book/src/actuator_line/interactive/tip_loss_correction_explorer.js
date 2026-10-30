// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Interactive plot of the tip loss correction function used in the actuator line model, used in
// the book. The plotting is done by the shared code in `sectional_models/interactive/model_explorer.js`.

(function () {
    'use strict';

    const { ModelExplorer, niceDomain } = window.StormbirdModelExplorer;

    // ---------------------------------------------------------------------------------------------
    // Parameters shown as sliders
    // ---------------------------------------------------------------------------------------------
    const CONTROL_GROUPS = [
        {
            name: 'Tip loss',
            parameters: [
                { key: 'exp_factor', min: 1.0, max: 50.0, step: 0.5 },
            ],
        },
    ];

    const DEFAULT_PARAMETERS = {
        exp_factor: 10.0,
    };

    const X_AXIS = {
        min: -0.5,
        max: 0.5,
        step: 0.001,
        ticks: { start: -0.5, step: 0.1, format: (s) => `${s}` },
    };

    // Same as `correction_factor` in the Rust implementation. The span position is limited to the
    // range of the wing, as the x-values in the plot can end slightly outside due to round off.
    function correctionFactor(p, s) {
        const expValue = Math.exp(-p.exp_factor * Math.max(0.5 - Math.abs(s), 0.0));

        return (2.0 / Math.PI) * Math.acos(expValue);
    }

    const PANELS = [
        {
            id: 'factor', height: 300, yLabel: 'Correction factor',
            ariaLabel: 'Tip loss correction factor as a function of the non-dimensional span distance',
            series: [
                { key: 'raw', label: 'Constant input, Γ_raw', className: 'me-component-dashed' },
                { key: 'scaled', label: 'Scaled to maintain the mean value, constant Γ_raw', className: 'me-series-secondary', hoverDot: true },
                { key: 'factor', label: 'Correction function, f_cor', className: 'me-series-primary', hoverDot: true },
            ],
            domain: (data) => niceDomain([...data.factor, ...data.scaled], 1.0),
            markers: () => [],
        },
    ];

    function compute(p, spanPositions) {
        const factor = spanPositions.map((s) => correctionFactor(p, s));

        // Mean value of the correction function over the span, using the trapezoidal rule
        let integral = 0.0;

        for (let i = 1; i < spanPositions.length; i++) {
            integral += 0.5 * (factor[i] + factor[i - 1]) * (spanPositions[i] - spanPositions[i - 1]);
        }

        const mean = integral / (X_AXIS.max - X_AXIS.min);

        return {
            raw: factor.map(() => 1.0),
            factor: factor,
            scaled: factor.map((value) => value / mean),
        };
    }

    function readout(s, d, index) {
        return [
            ['s', s.toFixed(3)],
            ['f_cor', d.factor[index].toFixed(3)],
            ['Scaled', d.scaled[index].toFixed(3)],
        ];
    }

    const CONFIG = {
        name: 'tip-loss-correction',
        defaultParameters: DEFAULT_PARAMETERS,
        initialParameters: {},
        controlGroups: CONTROL_GROUPS,
        xAxis: X_AXIS,
        panels: PANELS,
        compute: compute,
        readout: readout,
        hint: 'Hover over the plot to read values.',
    };

    for (const root of document.querySelectorAll('[data-tip-loss-correction-explorer]')) {
        new ModelExplorer(root, CONFIG);
    }
})();
