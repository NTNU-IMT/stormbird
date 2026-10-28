// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Interactive plot of the discrete deficiency function used in the dynamic stall model, used in
// the book. The input signal is a trapezoidal pulse, which becomes a triangle when the hold width
// is zero, and a rectangular pulse when the ramp width is zero. The plotting is done by the shared
// code in `model_explorer.js`.

(function () {
    'use strict';

    const { ModelExplorer, niceDomain } = window.StormbirdModelExplorer;

    // ---------------------------------------------------------------------------------------------
    // Parameters shown as sliders. All times are non-dimensional, in semi-chords travelled.
    // ---------------------------------------------------------------------------------------------
    const CONTROL_GROUPS = [
        {
            name: 'Lag',
            parameters: [
                { key: 'time_constant', min: 0.0, max: 10.0, step: 0.1 },
                { key: 'time_step', min: 0.05, max: 5.0, step: 0.05 },
            ],
        },
        {
            name: 'Input',
            parameters: [
                { key: 'pulse_start', min: 0.0, max: 20.0, step: 0.1 },
                { key: 'ramp_width', min: 0.0, max: 15.0, step: 0.1 },
                { key: 'hold_width', min: 0.0, max: 20.0, step: 0.1 },
            ],
        },
    ];

    // The input starts as a triangle
    const DEFAULT_PARAMETERS = {
        time_constant: 3.0,
        time_step: 0.5,
        pulse_start: 5.0,
        ramp_width: 4.0,
        hold_width: 0.0,
    };

    const X_AXIS = {
        min: 0.0,
        max: 40.0,
        step: 0.02,
        ticks: { start: 0.0, step: 5.0, format: (time) => `${time}` },
    };

    // The input signal, x(τ), with a max value of one
    function input(p, time) {
        const fallStart = p.pulse_start + p.ramp_width + p.hold_width;

        const fraction = (start) => {
            if (p.ramp_width > 0.0) {
                return Math.min(Math.max((time - start) / p.ramp_width, 0.0), 1.0);
            }

            return time >= start ? 1.0 : 0.0;
        };

        return fraction(p.pulse_start) - fraction(fallStart);
    }

    // Same as `lag_factors` in the Rust implementation. A time constant of zero turns the lag off.
    function lagFactors(deltaTime, timeConstant) {
        if (timeConstant <= 0.0) {
            return [0.0, 0.0];
        }

        return [
            Math.exp(-deltaTime / timeConstant),
            Math.exp(-deltaTime / (2.0 * timeConstant)),
        ];
    }

    function pulseMarkers(p) {
        const end = p.pulse_start + 2.0 * p.ramp_width + p.hold_width;

        return [
            { x: p.pulse_start, className: 'me-dashed-line' },
            { x: end, className: 'me-dashed-line' },
        ];
    }

    const PANELS = [
        {
            id: 'signal', height: 260, yLabel: 'Signal',
            ariaLabel: 'Input signal and lagged signal as a function of non-dimensional time',
            series: [
                { key: 'x', label: 'Input, x', className: 'me-component-dashed' },
                { key: 'exact', label: 'Continuous first order lag', className: 'me-component-dotted' },
                { key: 'lagged', label: 'Lagged signal, x − D', className: 'me-series-primary', hoverDot: true },
            ],
            domain: (data) => niceDomain([...data.x, ...data.lagged, ...data.exact], 1.0),
            markers: pulseMarkers,
        },
        {
            id: 'deficiency', height: 170, yLabel: 'Deficiency, D',
            ariaLabel: 'Deficiency function as a function of non-dimensional time',
            series: [
                { key: 'deficiency', label: 'Deficiency function, D', className: 'me-series-secondary', hoverDot: true },
            ],
            domain: (data) => niceDomain(data.deficiency, 0.5),
            markers: pulseMarkers,
        },
    ];

    function compute(p, times) {
        const deltaTime = p.time_step;
        const [decay, incrementWeight] = lagFactors(deltaTime, p.time_constant);

        // The discrete deficiency function, evaluated at the time steps
        const nrSteps = Math.floor(X_AXIS.max / deltaTime + 1e-9) + 1;

        const stepInput = [input(p, 0.0)];
        const stepDeficiency = [0.0];

        for (let i = 1; i < nrSteps; i++) {
            stepInput.push(input(p, i * deltaTime));

            stepDeficiency.push(
                stepDeficiency[i - 1] * decay + (stepInput[i] - stepInput[i - 1]) * incrementWeight
            );
        }

        const data = { x: [], lagged: [], exact: [], deficiency: [] };

        let exact = input(p, times[0]);
        let previousTime = times[0];
        let previousInput = exact;

        for (const time of times) {
            const x = input(p, time);

            // The continuous lag, dy/dτ = (x - y) / T, solved exactly for an input that varies
            // linearly between the points in the plot.
            const h = time - previousTime;

            if (p.time_constant <= 0.0) {
                exact = x;
            } else if (h > 0.0) {
                const slope = (x - previousInput) / h;
                const slopeTerm = slope * p.time_constant;

                exact = x - slopeTerm + (exact - previousInput + slopeTerm) * Math.exp(-h / p.time_constant);
            }

            previousTime = time;
            previousInput = x;

            // The discrete values are held constant until the next time step
            const stepIndex = Math.min(Math.floor(time / deltaTime + 1e-9), nrSteps - 1);

            data.x.push(x);
            data.exact.push(exact);
            data.lagged.push(stepInput[stepIndex] - stepDeficiency[stepIndex]);
            data.deficiency.push(stepDeficiency[stepIndex]);
        }

        return data;
    }

    function readout(time, d, index) {
        return [
            ['τ', time.toFixed(2)],
            ['x', d.x[index].toFixed(3)],
            ['x − D', d.lagged[index].toFixed(3)],
            ['Continuous lag', d.exact[index].toFixed(3)],
            ['D', d.deficiency[index].toFixed(3)],
        ];
    }

    const CONFIG = {
        name: 'deficiency-function',
        defaultParameters: DEFAULT_PARAMETERS,
        initialParameters: {},
        controlGroups: CONTROL_GROUPS,
        xAxis: X_AXIS,
        panels: PANELS,
        compute: compute,
        readout: readout,
        hint: 'Hover over the plots to read values. Dashed vertical lines: start and end of the input pulse. The discrete values are held constant through each time step.',
    };

    for (const root of document.querySelectorAll('[data-deficiency-function-explorer]')) {
        new ModelExplorer(root, CONFIG);
    }
})();
