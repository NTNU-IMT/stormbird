// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Interactive plot of the `Foil` model, used in the book. The model itself is in `foil_model.js`.
// The plot is drawn as plain SVG, without any external dependencies, so that the book can be
// published as a static web site.

(function () {
    'use strict';

    const { Foil, DEFAULT_PARAMETERS } = window.StormbirdFoilModel;

    const SVG_NS = 'http://www.w3.org/2000/svg';

    const toRadians = (degrees) => degrees * Math.PI / 180.0;
    const toDegrees = (radians) => radians * 180.0 / Math.PI;

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
                { key: 'cd_bump_during_stall', min: 0.0, max: 1.0, step: 0.01 },
                { key: 'cd_stall_angle_offset', min: -10.0, max: 10.0, step: 0.5, angle: true },
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

    const ANGLE_MIN = -40.0;
    const ANGLE_MAX = 95.0;
    const ANGLE_STEP = 0.25;

    // ---------------------------------------------------------------------------------------------
    // Small helpers
    // ---------------------------------------------------------------------------------------------
    function element(tag, attributes = {}, children = []) {
        const node = document.createElement(tag);

        for (const [name, value] of Object.entries(attributes)) {
            if (name === 'text') {
                node.textContent = value;
            } else {
                node.setAttribute(name, value);
            }
        }

        for (const child of children) {
            node.appendChild(child);
        }

        return node;
    }

    function svgElement(tag, attributes = {}) {
        const node = document.createElementNS(SVG_NS, tag);

        for (const [name, value] of Object.entries(attributes)) {
            node.setAttribute(name, value);
        }

        return node;
    }

    function decimalsOf(step) {
        const text = String(step);

        return text.includes('.') ? text.split('.')[1].length : 0;
    }

    function niceStep(span, targetTicks) {
        const rawStep = span / targetTicks;
        const magnitude = Math.pow(10, Math.floor(Math.log10(rawStep)));

        for (const factor of [1, 2, 2.5, 5, 10]) {
            if (factor * magnitude >= rawStep) {
                return factor * magnitude;
            }
        }

        return 10 * magnitude;
    }

    function niceDomain(values, minimumSpan) {
        let min = Math.min(0.0, ...values);
        let max = Math.max(0.0, ...values);

        if (max - min < minimumSpan) {
            max = min + minimumSpan;
        }

        const step = niceStep(max - min, 5);

        // Pad the limits slightly, except when the limit is zero, so that the zero line is used
        // as the edge of the plot when all values are positive or negative.
        return {
            min: min < 0.0 ? Math.floor(min / step - 0.05) * step : 0.0,
            max: max > 0.0 ? Math.ceil(max / step + 0.05) * step : 0.0,
            step: step,
        };
    }

    // ---------------------------------------------------------------------------------------------
    // Plot panel
    // ---------------------------------------------------------------------------------------------
    const WIDTH = 720;
    const MARGIN = { left: 52, right: 14, top: 10, bottom: 30 };

    class Panel {
        constructor(container, options) {
            this.height = options.height;
            this.yLabel = options.yLabel;
            this.showXLabel = options.showXLabel;
            this.series = options.series;

            const legend = element('div', { class: 'fe-legend' });

            for (const series of this.series) {
                const swatch = svgElement('svg', { width: 26, height: 10, 'aria-hidden': 'true' });

                swatch.appendChild(svgElement('line', {
                    x1: 1, y1: 5, x2: 25, y2: 5,
                    class: `fe-line ${series.className}`,
                }));

                legend.appendChild(element('span', { class: 'fe-legend-item' }, [
                    swatch, element('span', { text: series.label }),
                ]));
            }

            this.svg = svgElement('svg', {
                viewBox: `0 0 ${WIDTH} ${this.height}`,
                class: 'fe-plot',
                role: 'img',
                'aria-label': options.ariaLabel,
            });

            const clipId = `fe-clip-${options.id}`;
            const clip = svgElement('clipPath', { id: clipId });

            clip.appendChild(svgElement('rect', {
                x: MARGIN.left, y: MARGIN.top,
                width: WIDTH - MARGIN.left - MARGIN.right,
                height: this.height - MARGIN.top - MARGIN.bottom,
            }));

            const defs = svgElement('defs');
            defs.appendChild(clip);
            this.svg.appendChild(defs);

            this.axesLayer = svgElement('g');
            this.markerLayer = svgElement('g', { 'clip-path': `url(#${clipId})` });
            this.lineLayer = svgElement('g', { 'clip-path': `url(#${clipId})` });
            this.hoverLayer = svgElement('g', { class: 'fe-hover', visibility: 'hidden' });

            this.svg.append(this.axesLayer, this.markerLayer, this.lineLayer, this.hoverLayer);

            this.paths = this.series.map((series) => {
                const path = svgElement('path', { class: `fe-line ${series.className}` });
                this.lineLayer.appendChild(path);
                return path;
            });

            this.crosshair = svgElement('line', {
                class: 'fe-crosshair', y1: MARGIN.top, y2: this.height - MARGIN.bottom,
            });

            this.hoverLayer.appendChild(this.crosshair);

            this.hoverDots = this.series.filter((s) => s.hoverDot).map((series) => {
                const dot = svgElement('circle', { r: 4.5, class: `fe-dot ${series.className}` });
                this.hoverLayer.appendChild(dot);
                return { series, dot };
            });

            container.append(legend, this.svg);
        }

        xScale(angleDegrees) {
            return MARGIN.left + (angleDegrees - ANGLE_MIN) / (ANGLE_MAX - ANGLE_MIN) *
                (WIDTH - MARGIN.left - MARGIN.right);
        }

        xInverse(x) {
            return ANGLE_MIN + (x - MARGIN.left) / (WIDTH - MARGIN.left - MARGIN.right) *
                (ANGLE_MAX - ANGLE_MIN);
        }

        yScale(value) {
            const fraction = (value - this.domain.min) / (this.domain.max - this.domain.min);

            return this.height - MARGIN.bottom - fraction * (this.height - MARGIN.top - MARGIN.bottom);
        }

        drawAxes() {
            this.axesLayer.replaceChildren();

            const bottom = this.height - MARGIN.bottom;

            for (let value = this.domain.min; value <= this.domain.max + 1e-9; value += this.domain.step) {
                const y = this.yScale(value);
                const isZero = Math.abs(value) < 1e-9;

                this.axesLayer.appendChild(svgElement('line', {
                    x1: MARGIN.left, x2: WIDTH - MARGIN.right, y1: y, y2: y,
                    class: isZero ? 'fe-zero-line' : 'fe-grid',
                }));

                const label = svgElement('text', {
                    x: MARGIN.left - 6, y: y, class: 'fe-tick-label',
                    'text-anchor': 'end', 'dominant-baseline': 'middle',
                });

                label.textContent = Number(value.toFixed(6)).toString();
                this.axesLayer.appendChild(label);
            }

            for (let angle = -30; angle <= ANGLE_MAX; angle += 15) {
                const x = this.xScale(angle);

                this.axesLayer.appendChild(svgElement('line', {
                    x1: x, x2: x, y1: MARGIN.top, y2: bottom,
                    class: angle === 0 ? 'fe-zero-line' : 'fe-grid',
                }));

                const label = svgElement('text', {
                    x: x, y: bottom + 16, class: 'fe-tick-label', 'text-anchor': 'middle',
                });

                label.textContent = `${angle}°`;
                this.axesLayer.appendChild(label);
            }

            const yLabel = svgElement('text', {
                x: 12, y: (MARGIN.top + bottom) / 2, class: 'fe-axis-label',
                'text-anchor': 'middle',
                transform: `rotate(-90 12 ${(MARGIN.top + bottom) / 2})`,
            });

            yLabel.textContent = this.yLabel;
            this.axesLayer.appendChild(yLabel);
        }

        drawStallMarkers(markers) {
            this.markerLayer.replaceChildren();

            for (const marker of markers) {
                const x = this.xScale(marker.angle);

                if (marker.range !== undefined) {
                    const x0 = this.xScale(marker.angle - marker.range);
                    const x1 = this.xScale(marker.angle + marker.range);

                    this.markerLayer.appendChild(svgElement('rect', {
                        x: Math.min(x0, x1), width: Math.abs(x1 - x0),
                        y: MARGIN.top, height: this.height - MARGIN.top - MARGIN.bottom,
                        class: 'fe-stall-band',
                    }));
                }

                this.markerLayer.appendChild(svgElement('line', {
                    x1: x, x2: x, y1: MARGIN.top, y2: this.height - MARGIN.bottom,
                    class: marker.className,
                }));
            }
        }

        update(angles, data, domain, markers) {
            this.domain = domain;
            this.data = data;

            this.drawAxes();
            this.drawStallMarkers(markers);

            this.series.forEach((series, index) => {
                const values = data[series.key];

                let d = '';

                for (let i = 0; i < angles.length; i++) {
                    // Clamp far outside values, so that the clipped lines are drawn correctly
                    const y = Math.max(-1e4, Math.min(1e4, this.yScale(values[i])));

                    d += `${i === 0 ? 'M' : 'L'}${this.xScale(angles[i]).toFixed(2)},${y.toFixed(2)}`;
                }

                this.paths[index].setAttribute('d', d);
            });
        }

        showHover(index, angle) {
            const x = this.xScale(angle);

            this.crosshair.setAttribute('x1', x);
            this.crosshair.setAttribute('x2', x);

            for (const { series, dot } of this.hoverDots) {
                dot.setAttribute('cx', x);
                dot.setAttribute('cy', this.yScale(this.data[series.key][index]));
            }

            this.hoverLayer.setAttribute('visibility', 'visible');
        }

        hideHover() {
            this.hoverLayer.setAttribute('visibility', 'hidden');
        }
    }

    // ---------------------------------------------------------------------------------------------
    // Explorer
    // ---------------------------------------------------------------------------------------------
    class FoilModelExplorer {
        constructor(root) {
            this.root = root;
            this.parameters = { ...DEFAULT_PARAMETERS, ...INITIAL_PARAMETERS };
            this.inputs = {};

            this.angles = [];

            for (let angle = ANGLE_MIN; angle <= ANGLE_MAX + 1e-9; angle += ANGLE_STEP) {
                this.angles.push(angle);
            }

            this.buildLayout();
            this.update();
        }

        buildLayout() {
            this.root.classList.add('foil-explorer');
            this.root.replaceChildren();

            this.readout = element('div', { class: 'fe-readout', 'aria-live': 'polite' });
            this.root.appendChild(this.readout);

            const plots = element('div', { class: 'fe-plots' });
            this.root.appendChild(plots);

            this.panels = [
                new Panel(plots, {
                    id: 'lift', height: 250, yLabel: 'Lift coefficient',
                    ariaLabel: 'Lift coefficient as a function of angle of attack',
                    // The total is drawn first, so that the thin component lines are visible on
                    // top of it where they are equal
                    series: [
                        { key: 'cl', label: 'Lift coefficient', className: 'fe-series-lift', hoverDot: true },
                        { key: 'clPreStall', label: 'Pre-stall model', className: 'fe-component-pre' },
                        { key: 'clPostStall', label: 'Post-stall model', className: 'fe-component-post' },
                    ],
                }),
                new Panel(plots, {
                    id: 'drag', height: 190, yLabel: 'Drag coefficient',
                    ariaLabel: 'Drag coefficient as a function of angle of attack',
                    series: [
                        { key: 'cd', label: 'Drag coefficient', className: 'fe-series-drag', hoverDot: true },
                        { key: 'cdPreStall', label: 'Pre-stall model', className: 'fe-component-pre' },
                        { key: 'cdPostStall', label: 'Post-stall model', className: 'fe-component-post' },
                    ],
                }),
                new Panel(plots, {
                    id: 'stall', height: 130, yLabel: 'Amount of stall',
                    ariaLabel: 'Amount of stall as a function of angle of attack',
                    series: [
                        { key: 'amountOfStall', label: 'Lift, s', className: 'fe-series-lift', hoverDot: true },
                        { key: 'amountOfStallDrag', label: 'Drag, s_D', className: 'fe-series-drag fe-dashed', hoverDot: true },
                    ],
                }),
            ];

            for (const panel of this.panels) {
                panel.svg.addEventListener('pointermove', (event) => this.onPointerMove(event, panel));
                panel.svg.addEventListener('pointerleave', () => this.onPointerLeave());
            }

            this.buildControls();
        }

        buildControls() {
            const controls = element('div', { class: 'fe-controls' });

            const tabList = element('div', { class: 'fe-tabs', role: 'tablist' });
            const tabPanels = [];
            const tabs = [];

            CONTROL_GROUPS.forEach((group, groupIndex) => {
                const tab = element('button', {
                    type: 'button', role: 'tab', class: 'fe-tab',
                    'aria-selected': groupIndex === 0 ? 'true' : 'false',
                    text: group.name,
                });

                const tabPanel = element('div', { class: 'fe-tab-panel', role: 'tabpanel' });

                if (groupIndex !== 0) {
                    tabPanel.hidden = true;
                }

                for (const parameter of group.parameters) {
                    tabPanel.appendChild(this.buildSlider(parameter));
                }

                tab.addEventListener('click', () => {
                    tabs.forEach((t, i) => t.setAttribute('aria-selected', i === groupIndex ? 'true' : 'false'));
                    tabPanels.forEach((p, i) => { p.hidden = i !== groupIndex; });
                });

                tabs.push(tab);
                tabPanels.push(tabPanel);
                tabList.appendChild(tab);
            });

            const resetButton = element('button', { type: 'button', class: 'fe-button', text: 'Reset' });

            resetButton.addEventListener('click', () => {
                this.parameters = { ...DEFAULT_PARAMETERS, ...INITIAL_PARAMETERS };
                this.syncInputs();
                this.update();
            });

            tabList.appendChild(element('span', { class: 'fe-spacer' }));
            tabList.appendChild(resetButton);

            controls.appendChild(tabList);
            tabPanels.forEach((p) => controls.appendChild(p));

            this.json = element('pre', { class: 'fe-json' });

            const copyButton = element('button', { type: 'button', class: 'fe-button', text: 'Copy' });

            copyButton.addEventListener('click', () => {
                navigator.clipboard.writeText(this.json.textContent).then(() => {
                    copyButton.textContent = 'Copied';
                    setTimeout(() => { copyButton.textContent = 'Copy'; }, 1500);
                });
            });

            const details = element('details', { class: 'fe-details' }, [
                element('summary', { text: 'Parameters as JSON input (angles in radians)' }),
                copyButton,
                this.json,
            ]);

            controls.appendChild(details);

            this.root.appendChild(controls);
        }

        buildSlider(parameter) {
            const fromModel = (value) => parameter.angle ? toDegrees(value) : value;
            const toModel = (value) => parameter.angle ? toRadians(value) : value;

            const decimals = decimalsOf(parameter.step);
            const id = `fe-input-${parameter.key}`;

            const range = element('input', {
                type: 'range', id: id,
                min: parameter.min, max: parameter.max, step: parameter.step,
            });

            const number = element('input', {
                type: 'number', class: 'fe-number', step: parameter.step,
                'aria-label': `${parameter.key} value`,
            });

            const unit = element('span', { class: 'fe-unit', text: parameter.angle ? '°' : '' });

            const set = (value, source) => {
                if (!Number.isFinite(value)) {
                    return;
                }

                this.parameters[parameter.key] = toModel(value);

                if (source !== range) {
                    range.value = value;
                }

                if (source !== number) {
                    number.value = value.toFixed(decimals);
                }

                this.update();
            };

            range.addEventListener('input', () => set(parseFloat(range.value), range));
            number.addEventListener('change', () => set(parseFloat(number.value), number));

            this.inputs[parameter.key] = () => {
                const value = fromModel(this.parameters[parameter.key]);

                range.value = value;
                number.value = value.toFixed(decimals);
            };

            this.inputs[parameter.key]();

            return element('div', { class: 'fe-slider' }, [
                element('label', { for: id, text: parameter.key }),
                element('div', { class: 'fe-slider-inputs' }, [range, number, unit]),
            ]);
        }

        syncInputs() {
            for (const sync of Object.values(this.inputs)) {
                sync();
            }
        }

        update() {
            const foil = new Foil(this.parameters);

            const data = {
                cl: [], clPreStall: [], clPostStall: [],
                cd: [], cdPreStall: [], cdPostStall: [],
                amountOfStall: [], amountOfStallDrag: [],
            };

            for (const angle of this.angles) {
                const a = toRadians(angle);

                data.cl.push(foil.liftCoefficient(a));
                data.clPreStall.push(foil.liftCoefficientPreStallRaw(a));
                data.clPostStall.push(foil.liftCoefficientPostStallRaw(a));
                data.cd.push(foil.dragCoefficient(a));
                data.cdPreStall.push(foil.dragCoefficientPreStall(a));
                data.cdPostStall.push(foil.dragCoefficientPostStallRaw(a));
                data.amountOfStall.push(foil.amountOfStall(a));
                data.amountOfStallDrag.push(foil.amountOfStallDrag(a));
            }

            this.data = data;

            // The axis limits follow the total values and the post-stall values. The pre-stall
            // values grow without bounds for large angles, and are therefore clipped.
            const liftDomain = niceDomain([...data.cl, ...data.clPostStall], 2.0);
            const dragDomain = niceDomain([...data.cd, ...data.cdPostStall], 0.5);
            const stallDomain = { min: 0.0, max: 1.0, step: 0.5 };

            const p = this.parameters;

            const stallMarkers = [
                { angle: toDegrees(Math.abs(p.mean_positive_stall_angle)), range: toDegrees(p.stall_range), className: 'fe-stall-line' },
                { angle: -toDegrees(Math.abs(p.mean_negative_stall_angle)), range: toDegrees(p.stall_range), className: 'fe-stall-line' },
            ];

            const onsetMarkers = [];

            if (p.cl_slope_reduction_factor_positive !== 0.0) {
                onsetMarkers.push({ angle: toDegrees(Math.abs(p.cl_slope_reduction_onset_angle_positive)), className: 'fe-onset-line' });
            }

            if (p.cl_slope_reduction_factor_negative !== 0.0) {
                onsetMarkers.push({ angle: -toDegrees(Math.abs(p.cl_slope_reduction_onset_angle_negative)), className: 'fe-onset-line' });
            }

            this.panels[0].update(this.angles, data, liftDomain, [...stallMarkers, ...onsetMarkers]);
            this.panels[1].update(this.angles, data, dragDomain, stallMarkers);
            this.panels[2].update(this.angles, data, stallDomain, stallMarkers);

            this.updateJson();

            if (this.hoverIndex !== undefined) {
                this.showHover(this.hoverIndex);
            } else {
                this.clearReadout();
            }
        }

        updateJson() {
            const output = {};

            for (const group of CONTROL_GROUPS) {
                for (const parameter of group.parameters) {
                    output[parameter.key] = Number(this.parameters[parameter.key].toPrecision(6));
                }
            }

            this.json.textContent = JSON.stringify(output, null, 4);
        }

        onPointerMove(event, panel) {
            const point = new DOMPoint(event.clientX, event.clientY)
                .matrixTransform(panel.svg.getScreenCTM().inverse());

            const angle = Math.max(ANGLE_MIN, Math.min(ANGLE_MAX, panel.xInverse(point.x)));

            this.hoverIndex = Math.round((angle - ANGLE_MIN) / ANGLE_STEP);
            this.showHover(this.hoverIndex);
        }

        onPointerLeave() {
            this.hoverIndex = undefined;

            for (const panel of this.panels) {
                panel.hideHover();
            }

            this.clearReadout();
        }

        showHover(index) {
            const angle = this.angles[index];

            for (const panel of this.panels) {
                panel.showHover(index, angle);
            }

            const d = this.data;

            this.readout.replaceChildren(
                this.readoutItem('α', `${angle.toFixed(2)}°`),
                this.readoutItem('C_L', d.cl[index].toFixed(3)),
                this.readoutItem('C_D', d.cd[index].toFixed(4)),
                this.readoutItem('C_L / C_D', Math.abs(d.cd[index]) > 1e-12 ? (d.cl[index] / d.cd[index]).toFixed(1) : '–'),
                this.readoutItem('s', d.amountOfStall[index].toFixed(3)),
                this.readoutItem('s_D', d.amountOfStallDrag[index].toFixed(3)),
            );
        }

        readoutItem(name, value) {
            return element('span', { class: 'fe-readout-item' }, [
                element('span', { class: 'fe-readout-name', text: name }),
                element('span', { class: 'fe-readout-value', text: value }),
            ]);
        }

        clearReadout() {
            this.readout.replaceChildren(element('span', {
                class: 'fe-readout-hint',
                text: 'Hover over the plots to read values. Dashed vertical lines: mean stall angles, with the stall range shaded. Dotted: slope reduction onset.',
            }));
        }
    }

    for (const root of document.querySelectorAll('[data-foil-model-explorer]')) {
        new FoilModelExplorer(root);
    }
})();
