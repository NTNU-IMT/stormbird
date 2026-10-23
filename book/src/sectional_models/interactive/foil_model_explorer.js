// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Interactive plot of the `Foil` model, used in the book. The model itself is in `foil_model.js`,
// while the plotting is done by the shared code in `model_explorer.js`.

(function () {
    'use strict';

    const { Foil, DEFAULT_PARAMETERS } = window.StormbirdFoilModel;
    const { ModelExplorer, niceDomain, toRadians, toDegrees } = window.StormbirdModelExplorer;

    // ---------------------------------------------------------------------------------------------
    // Parameters shown as sliders. Angles are given in degrees in the user interface, but are
    // stored in radians, as in the Rust model.
    // ---------------------------------------------------------------------------------------------
    const CONTROL_GROUPS = [
        {
            name: 'Lift',
            parameters: [
                { key: 'cl_zero_angle', min: -2.0, max: 4.0, step: 0.05 },
                { key: 'cl_initial_slope', min: 0.0, max: 10.0, step: 0.05 },
                { key: 'cl_slope_reduction_factor_positive', min: 0.0, max: 1.0, step: 0.01 },
                { key: 'cl_slope_reduction_factor_negative', min: 0.0, max: 1.0, step: 0.01 },
                { key: 'cl_slope_reduction_onset_angle_positive', min: 0.0, max: 30.0, step: 0.5, angle: true },
                { key: 'cl_slope_reduction_onset_angle_negative', min: 0.0, max: 30.0, step: 0.5, angle: true },
                { key: 'cl_slope_reduction_range', min: 0.0, max: 15.0, step: 0.5, angle: true },
                { key: 'cl_max_after_stall', min: 0.0, max: 2.0, step: 0.05 },
            ],
        },
        {
            name: 'Drag',
            parameters: [
                { key: 'cd_min', min: 0.0, max: 0.1, step: 0.001 },
                { key: 'cd_second_order_factor', min: 0.0, max: 3.0, step: 0.05 },
                { key: 'cd_max_after_stall', min: 0.0, max: 2.5, step: 0.05 },
                { key: 'cd_power_after_stall', min: 0.5, max: 4.0, step: 0.1 },
            ],
        },
        {
            name: 'Stall',
            parameters: [
                { key: 'mean_positive_stall_angle', min: 0.0, max: 45.0, step: 0.5, angle: true },
                { key: 'mean_negative_stall_angle', min: 0.0, max: 45.0, step: 0.5, angle: true },
                { key: 'stall_range', min: 0.5, max: 20.0, step: 0.5, angle: true },
            ],
        },
    ];

    // Initial values. Parameters not given here use the defaults of the Rust model.
    const INITIAL_PARAMETERS = {
        cl_zero_angle: 0.5,
        cl_slope_reduction_factor_positive: 0.3,
        cl_slope_reduction_onset_angle_positive: toRadians(12.0),
        cl_max_after_stall: 0.9,
        cd_min: 0.01,
        cd_second_order_factor: 0.5,
        cd_max_after_stall: 1.2,
        mean_positive_stall_angle: toRadians(20.0),
        mean_negative_stall_angle: toRadians(16.0),
        stall_range: toRadians(6.0),
    };

    // The angle of attack is given in degrees on the x-axis
    const X_AXIS = {
        min: -40.0,
        max: 95.0,
        step: 0.25,
        ticks: { start: -30.0, step: 15.0, format: (angle) => `${angle}°` },
    };

    function stallMarkers(p) {
        return [
            { x: toDegrees(Math.abs(p.mean_positive_stall_angle)), range: toDegrees(p.stall_range), className: 'me-dashed-line' },
            { x: -toDegrees(Math.abs(p.mean_negative_stall_angle)), range: toDegrees(p.stall_range), className: 'me-dashed-line' },
        ];
    }

    function onsetMarkers(p) {
        const markers = [];

        if (p.cl_slope_reduction_factor_positive !== 0.0) {
            markers.push({ x: toDegrees(Math.abs(p.cl_slope_reduction_onset_angle_positive)), className: 'me-dotted-line' });
        }

        if (p.cl_slope_reduction_factor_negative !== 0.0) {
            markers.push({ x: -toDegrees(Math.abs(p.cl_slope_reduction_onset_angle_negative)), className: 'me-dotted-line' });
        }

        return markers;
    }

    const PANELS = [
        {
            id: 'lift', height: 250, yLabel: 'Lift coefficient',
            ariaLabel: 'Lift coefficient as a function of angle of attack',
            // The total is drawn first, so that the thin component lines are visible on top of it
            // where they are equal
            series: [
                { key: 'cl', label: 'Lift coefficient', className: 'me-series-primary', hoverDot: true },
                { key: 'clPreStall', label: 'Pre-stall model', className: 'me-component-dashed' },
                { key: 'clPostStall', label: 'Post-stall model', className: 'me-component-dotted' },
            ],
            // The axis limits follow the total values and the post-stall values. The pre-stall
            // values grow without bounds for large angles, and are therefore clipped.
            domain: (data) => niceDomain([...data.cl, ...data.clPostStall], 2.0),
            markers: (p) => [...stallMarkers(p), ...onsetMarkers(p)],
        },
        {
            id: 'drag', height: 190, yLabel: 'Drag coefficient',
            ariaLabel: 'Drag coefficient as a function of angle of attack',
            series: [
                { key: 'cd', label: 'Drag coefficient', className: 'me-series-secondary', hoverDot: true },
                { key: 'cdPreStall', label: 'Pre-stall model', className: 'me-component-dashed' },
                { key: 'cdPostStall', label: 'Post-stall model', className: 'me-component-dotted' },
            ],
            domain: (data) => niceDomain([...data.cd, ...data.cdPostStall], 0.5),
            markers: stallMarkers,
        },
        {
            id: 'stall', height: 130, yLabel: 'Amount of stall',
            ariaLabel: 'Amount of stall as a function of angle of attack',
            series: [
                { key: 'amountOfStall', label: 'Amount of stall, s', className: 'me-series-primary', hoverDot: true },
            ],
            domain: () => ({ min: 0.0, max: 1.0, step: 0.5 }),
            markers: stallMarkers,
        },
    ];

    function compute(parameters, angles) {
        const foil = new Foil(parameters);

        const data = {
            cl: [], clPreStall: [], clPostStall: [],
            cd: [], cdPreStall: [], cdPostStall: [],
            amountOfStall: [],
        };

        for (const angle of angles) {
            const a = toRadians(angle);

            data.cl.push(foil.liftCoefficient(a));
            data.clPreStall.push(foil.liftCoefficientPreStallRaw(a));
            data.clPostStall.push(foil.liftCoefficientPostStallRaw(a));
            data.cd.push(foil.dragCoefficient(a));
            data.cdPreStall.push(foil.dragCoefficientPreStall(a));
            data.cdPostStall.push(foil.dragCoefficientPostStallRaw(a));
            data.amountOfStall.push(foil.amountOfStall(a));
        }

        return data;
    }

    function readout(angle, d, index) {
        return [
            ['α', `${angle.toFixed(2)}°`],
            ['C_L', d.cl[index].toFixed(3)],
            ['C_D', d.cd[index].toFixed(4)],
            ['C_L / C_D', Math.abs(d.cd[index]) > 1e-12 ? (d.cl[index] / d.cd[index]).toFixed(1) : '–'],
            ['s', d.amountOfStall[index].toFixed(3)],
        ];
    }

    const CONFIG = {
        name: 'foil',
        defaultParameters: DEFAULT_PARAMETERS,
        initialParameters: INITIAL_PARAMETERS,
        controlGroups: CONTROL_GROUPS,
        xAxis: X_AXIS,
        panels: PANELS,
        compute: compute,
        readout: readout,
        hint: 'Hover over the plots to read values. Dashed vertical lines: mean stall angles, with the stall range shaded. Dotted: slope reduction onset.',
        jsonSummary: 'Parameters as JSON input (angles in radians)',
    };

    for (const root of document.querySelectorAll('[data-foil-model-explorer]')) {
        new ModelExplorer(root, CONFIG);
    }
})();
