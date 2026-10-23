// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Interactive plot of the `RotatingCylinder` model, used in the book. The model itself is in
// `rotating_cylinder_model.js`, while the plotting is done by the shared code in
// `model_explorer.js`.

(function () {
    'use strict';

    const { RotatingCylinder, DEFAULT_PARAMETERS } = window.StormbirdRotatingCylinderModel;
    const { ModelExplorer, niceDomain } = window.StormbirdModelExplorer;

    // ---------------------------------------------------------------------------------------------
    // Parameters shown as sliders. Angles are given in degrees in the user interface, but are
    // stored in radians, as in the Rust model.
    // ---------------------------------------------------------------------------------------------
    const CONTROL_GROUPS = [
        {
            name: 'Lift',
            parameters: [
                { key: 'cl_initial_slope', min: 0.0, max: 6.0, step: 0.05 },
                { key: 'cl_high_spin_ratio_slope', min: -1.0, max: 6.0, step: 0.05 },
                { key: 'cl_slope_transition_spin_ratio', min: 0.0, max: 5.0, step: 0.05 },
                { key: 'cl_slope_transition_range', min: 0.0, max: 3.0, step: 0.05 },
            ],
        },
        {
            name: 'Drag',
            parameters: [
                { key: 'cd_start', min: 0.0, max: 1.5, step: 0.005 },
                { key: 'cd_end', min: 0.0, max: 0.5, step: 0.005 },
                { key: 'cd_angle_start', min: 0.0, max: 45.0, step: 0.5, angle: true },
                { key: 'cd_spin_ratio_low_drag', min: 0.5, max: 5.0, step: 0.05 },
            ],
        },
    ];

    // The plot starts with the defaults of the Rust model
    const INITIAL_PARAMETERS = {};

    // Higher spin ratios are typically not relevant for rotor sails
    const X_AXIS = {
        min: -5.0,
        max: 5.0,
        step: 0.02,
        ticks: { start: -5.0, step: 1.0, format: (spinRatio) => `${spinRatio}` },
    };

    // Step used when computing the lift slope numerically
    const SLOPE_DELTA = 1e-5;

    function transitionMarkers(p) {
        const transitionSpinRatio = Math.abs(p.cl_slope_transition_spin_ratio);

        return [
            { x: transitionSpinRatio, range: p.cl_slope_transition_range, className: 'me-dashed-line' },
            { x: -transitionSpinRatio, range: p.cl_slope_transition_range, className: 'me-dashed-line' },
        ];
    }

    function lowDragMarkers(p) {
        return [
            { x: p.cd_spin_ratio_low_drag, className: 'me-dotted-line' },
            { x: -p.cd_spin_ratio_low_drag, className: 'me-dotted-line' },
        ];
    }

    const PANELS = [
        {
            id: 'lift', height: 250, yLabel: 'Lift coefficient',
            ariaLabel: 'Lift coefficient as a function of spin ratio',
            // The total is drawn first, so that the thin component lines are visible on top of it
            // where they are equal
            series: [
                { key: 'cl', label: 'Lift coefficient', className: 'me-series-primary', hoverDot: true },
                { key: 'clInitialSlope', label: 'Initial slope', className: 'me-component-dashed' },
                { key: 'clHighSpinRatio', label: 'High spin ratio asymptote', className: 'me-component-dotted' },
            ],
            // The axis limits follow the lift coefficient, while the straight lines are clipped
            domain: (data) => niceDomain(data.cl, 2.0),
            markers: transitionMarkers,
        },
        {
            id: 'slope', height: 150, yLabel: 'Lift slope',
            ariaLabel: 'Slope of the lift coefficient as a function of spin ratio',
            series: [
                { key: 'clSlope', label: 'dC_L / dλ', className: 'me-series-primary', hoverDot: true },
            ],
            domain: (data) => niceDomain(data.clSlope, 1.0),
            markers: transitionMarkers,
        },
        {
            id: 'drag', height: 190, yLabel: 'Drag coefficient',
            ariaLabel: 'Drag coefficient as a function of spin ratio',
            series: [
                { key: 'cd', label: 'Drag coefficient', className: 'me-series-secondary', hoverDot: true },
            ],
            domain: (data) => niceDomain(data.cd, 0.5),
            markers: lowDragMarkers,
        },
    ];

    function compute(parameters, spinRatios) {
        const cylinder = new RotatingCylinder(parameters);

        const initialSlope = parameters.cl_initial_slope;
        const highSpinRatioSlope = parameters.cl_high_spin_ratio_slope;
        const transitionSpinRatio = Math.abs(parameters.cl_slope_transition_spin_ratio);

        // Well above the transition, the lift approaches a straight line with the high spin ratio
        // slope: C_L = a_inf * |λ| + (a_0 - a_inf) * λ_t, with the sign of the spin ratio. It is only
        // drawn from the start of the transition, to keep the plot clean at low spin ratios.
        const asymptoteStart = Math.max(
            transitionSpinRatio - parameters.cl_slope_transition_range, X_AXIS.step
        );

        const data = { cl: [], clInitialSlope: [], clHighSpinRatio: [], clSlope: [], cd: [] };

        for (const spinRatio of spinRatios) {
            data.cl.push(cylinder.liftCoefficient(spinRatio));
            data.clInitialSlope.push(initialSlope * spinRatio);

            data.clHighSpinRatio.push(Math.abs(spinRatio) >= asymptoteStart ?
                Math.sign(spinRatio) * (
                    highSpinRatioSlope * Math.abs(spinRatio) +
                    (initialSlope - highSpinRatioSlope) * transitionSpinRatio
                ) :
                NaN
            );

            data.clSlope.push((
                cylinder.liftCoefficient(spinRatio + SLOPE_DELTA) -
                cylinder.liftCoefficient(spinRatio - SLOPE_DELTA)
            ) / (2.0 * SLOPE_DELTA));

            data.cd.push(cylinder.dragCoefficient(spinRatio));
        }

        return data;
    }

    function readout(spinRatio, d, index) {
        return [
            ['λ', spinRatio.toFixed(2)],
            ['C_L', d.cl[index].toFixed(3)],
            ['dC_L / dλ', d.clSlope[index].toFixed(3)],
            ['C_D', d.cd[index].toFixed(4)],
            ['C_L / C_D', Math.abs(d.cd[index]) > 1e-12 ? (d.cl[index] / d.cd[index]).toFixed(1) : '–'],
        ];
    }

    const CONFIG = {
        name: 'rotating-cylinder',
        defaultParameters: DEFAULT_PARAMETERS,
        initialParameters: INITIAL_PARAMETERS,
        controlGroups: CONTROL_GROUPS,
        xAxis: X_AXIS,
        panels: PANELS,
        compute: compute,
        readout: readout,
        hint: 'Hover over the plots to read values. Dashed vertical lines: transition spin ratio for the lift slope, with the transition range shaded. Dotted: spin ratio where the drag reaches cd_end.',
        jsonSummary: 'Parameters as JSON input (angles in radians)',
    };

    for (const root of document.querySelectorAll('[data-rotating-cylinder-model-explorer]')) {
        new ModelExplorer(root, CONFIG);
    }
})();
