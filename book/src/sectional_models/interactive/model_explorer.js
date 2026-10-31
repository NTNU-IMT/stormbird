// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

// Shared code for the interactive plots of the section models in the book. Each section model has
// its own explorer file, which sets up the parameters, the panels and the data to plot, and passes
// this to the `ModelExplorer` class below. The plots are drawn as plain SVG, without any external
// dependencies, so that the book can be published as a static web site.

(function () {
    'use strict';

    const SVG_NS = 'http://www.w3.org/2000/svg';

    const toRadians = (degrees) => degrees * Math.PI / 180.0;
    const toDegrees = (radians) => radians * 180.0 / Math.PI;

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

    // Axis limits and tick step for a set of values. Values that are not finite are ignored.
    function niceDomain(values, minimumSpan) {
        const finiteValues = values.filter(Number.isFinite);

        let min = Math.min(0.0, ...finiteValues);
        let max = Math.max(0.0, ...finiteValues);

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
        constructor(container, xAxis, options) {
            this.xAxis = xAxis;
            this.height = options.height;
            this.yLabel = options.yLabel;
            this.series = options.series;

            const legend = element('div', { class: 'me-legend' });

            for (const series of this.series) {
                const swatch = svgElement('svg', { width: 26, height: 10, 'aria-hidden': 'true' });

                swatch.appendChild(svgElement('line', {
                    x1: 1, y1: 5, x2: 25, y2: 5,
                    class: `me-line ${series.className}`,
                }));

                legend.appendChild(element('span', { class: 'me-legend-item' }, [
                    swatch, element('span', { text: series.label }),
                ]));
            }

            this.svg = svgElement('svg', {
                viewBox: `0 0 ${WIDTH} ${this.height}`,
                class: 'me-plot',
                role: 'img',
                'aria-label': options.ariaLabel,
            });

            const clipId = `me-clip-${options.id}`;
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
            this.hoverLayer = svgElement('g', { class: 'me-hover', visibility: 'hidden' });

            this.svg.append(this.axesLayer, this.markerLayer, this.lineLayer, this.hoverLayer);

            this.paths = this.series.map((series) => {
                const path = svgElement('path', { class: `me-line ${series.className}` });
                this.lineLayer.appendChild(path);
                return path;
            });

            this.crosshair = svgElement('line', {
                class: 'me-crosshair', y1: MARGIN.top, y2: this.height - MARGIN.bottom,
            });

            this.hoverLayer.appendChild(this.crosshair);

            this.hoverDots = this.series.filter((s) => s.hoverDot).map((series) => {
                const dot = svgElement('circle', { r: 4.5, class: `me-dot ${series.className}` });
                this.hoverLayer.appendChild(dot);
                return { series, dot };
            });

            container.append(legend, this.svg);
        }

        xScale(x) {
            return MARGIN.left + (x - this.xAxis.min) / (this.xAxis.max - this.xAxis.min) *
                (WIDTH - MARGIN.left - MARGIN.right);
        }

        xInverse(x) {
            return this.xAxis.min + (x - MARGIN.left) / (WIDTH - MARGIN.left - MARGIN.right) *
                (this.xAxis.max - this.xAxis.min);
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
                    class: isZero ? 'me-zero-line' : 'me-grid',
                }));

                const label = svgElement('text', {
                    x: MARGIN.left - 6, y: y, class: 'me-tick-label',
                    'text-anchor': 'end', 'dominant-baseline': 'middle',
                });

                label.textContent = Number(value.toFixed(6)).toString();
                this.axesLayer.appendChild(label);
            }

            const ticks = this.xAxis.ticks;

            for (let tick = ticks.start; tick <= this.xAxis.max + 1e-9; tick += ticks.step) {
                const x = this.xScale(tick);

                this.axesLayer.appendChild(svgElement('line', {
                    x1: x, x2: x, y1: MARGIN.top, y2: bottom,
                    class: Math.abs(tick) < 1e-9 ? 'me-zero-line' : 'me-grid',
                }));

                const label = svgElement('text', {
                    x: x, y: bottom + 16, class: 'me-tick-label', 'text-anchor': 'middle',
                });

                label.textContent = ticks.format(Number(tick.toFixed(6)));
                this.axesLayer.appendChild(label);
            }

            const yLabel = svgElement('text', {
                x: 12, y: (MARGIN.top + bottom) / 2, class: 'me-axis-label',
                'text-anchor': 'middle',
                transform: `rotate(-90 12 ${(MARGIN.top + bottom) / 2})`,
            });

            yLabel.textContent = this.yLabel;
            this.axesLayer.appendChild(yLabel);
        }

        // Vertical marker lines, at a position on the x-axis, with an optional shaded band of
        // plus/minus the range around the position.
        drawMarkers(markers) {
            this.markerLayer.replaceChildren();

            for (const marker of markers) {
                const x = this.xScale(marker.x);

                if (marker.range !== undefined) {
                    const x0 = this.xScale(marker.x - marker.range);
                    const x1 = this.xScale(marker.x + marker.range);

                    this.markerLayer.appendChild(svgElement('rect', {
                        x: Math.min(x0, x1), width: Math.abs(x1 - x0),
                        y: MARGIN.top, height: this.height - MARGIN.top - MARGIN.bottom,
                        class: 'me-band',
                    }));
                }

                this.markerLayer.appendChild(svgElement('line', {
                    x1: x, x2: x, y1: MARGIN.top, y2: this.height - MARGIN.bottom,
                    class: marker.className,
                }));
            }
        }

        update(xValues, data, domain, markers) {
            this.domain = domain;
            this.data = data;

            this.drawAxes();
            this.drawMarkers(markers);

            this.series.forEach((series, index) => {
                const values = data[series.key];

                let d = '';
                let newLine = true;

                for (let i = 0; i < xValues.length; i++) {
                    // Values that are not finite break the line
                    if (!Number.isFinite(values[i])) {
                        newLine = true;
                        continue;
                    }

                    // Clamp far outside values, so that the clipped lines are drawn correctly
                    const y = Math.max(-1e4, Math.min(1e4, this.yScale(values[i])));

                    d += `${newLine ? 'M' : 'L'}${this.xScale(xValues[i]).toFixed(2)},${y.toFixed(2)}`;
                    newLine = false;
                }

                this.paths[index].setAttribute('d', d);
            });
        }

        showHover(index, x) {
            const xPlot = this.xScale(x);

            this.crosshair.setAttribute('x1', xPlot);
            this.crosshair.setAttribute('x2', xPlot);

            for (const { series, dot } of this.hoverDots) {
                dot.setAttribute('cx', xPlot);
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

    // Interactive plot of a section model. The `config` object defines the model specific parts:
    //
    // - `name`: unique name of the explorer, used to make unique ids for the elements
    // - `defaultParameters` and `initialParameters`: the defaults of the Rust model, and the values
    //   used when the plot is first shown
    // - `controlGroups`: the parameters shown as sliders, in tabs. Parameters with `angle: true`
    //   are given in degrees in the user interface, but stored in radians, as in the Rust model
    // - `xAxis`: the range and step of the x-values, and the ticks on the x-axis
    // - `panels`: the plot panels, each with a list of series, and functions for the y-axis
    //   limits and the vertical markers
    // - `compute(parameters, xValues)`: returns the data for all series, as arrays
    // - `readout(x, data, index)`: returns the values shown when hovering, as [name, value] pairs
    // - `hint`: text shown when not hovering
    // - `jsonSummary`: title of the JSON output. The JSON output is not shown if this is not set
    class ModelExplorer {
        constructor(root, config) {
            this.root = root;
            this.config = config;
            this.parameters = { ...config.defaultParameters, ...config.initialParameters };
            this.inputs = {};

            const xAxis = config.xAxis;

            this.xValues = [];

            for (let x = xAxis.min; x <= xAxis.max + 1e-9; x += xAxis.step) {
                this.xValues.push(x);
            }

            this.buildLayout();
            this.update();
        }

        buildLayout() {
            this.root.classList.add('model-explorer');
            this.root.replaceChildren();

            this.readout = element('div', { class: 'me-readout', 'aria-live': 'polite' });
            this.root.appendChild(this.readout);

            const plots = element('div', { class: 'me-plots' });
            this.root.appendChild(plots);

            this.panels = this.config.panels.map((panelConfig) => new Panel(
                plots, this.config.xAxis, { ...panelConfig, id: `${this.config.name}-${panelConfig.id}` }
            ));

            for (const panel of this.panels) {
                panel.svg.addEventListener('pointermove', (event) => this.onPointerMove(event, panel));
                panel.svg.addEventListener('pointerleave', () => this.onPointerLeave());
            }

            this.buildControls();
        }

        buildControls() {
            const controls = element('div', { class: 'me-controls' });

            const tabList = element('div', { class: 'me-tabs', role: 'tablist' });
            const tabPanels = [];
            const tabs = [];

            this.config.controlGroups.forEach((group, groupIndex) => {
                const tab = element('button', {
                    type: 'button', role: 'tab', class: 'me-tab',
                    'aria-selected': groupIndex === 0 ? 'true' : 'false',
                    text: group.name,
                });

                const tabPanel = element('div', { class: 'me-tab-panel', role: 'tabpanel' });

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

            const resetButton = element('button', { type: 'button', class: 'me-button', text: 'Reset' });

            resetButton.addEventListener('click', () => {
                this.parameters = { ...this.config.defaultParameters, ...this.config.initialParameters };
                this.syncInputs();
                this.update();
            });

            tabList.appendChild(element('span', { class: 'me-spacer' }));
            tabList.appendChild(resetButton);

            controls.appendChild(tabList);
            tabPanels.forEach((p) => controls.appendChild(p));

            if (this.config.jsonSummary !== undefined) {
                this.buildJsonOutput(controls);
            }

            this.root.appendChild(controls);
        }

        buildJsonOutput(controls) {
            this.json = element('pre', { class: 'me-json' });

            const copyButton = element('button', { type: 'button', class: 'me-button', text: 'Copy' });

            copyButton.addEventListener('click', () => {
                navigator.clipboard.writeText(this.json.textContent).then(() => {
                    copyButton.textContent = 'Copied';
                    setTimeout(() => { copyButton.textContent = 'Copy'; }, 1500);
                });
            });

            const details = element('details', { class: 'me-details' }, [
                element('summary', { text: this.config.jsonSummary }),
                copyButton,
                this.json,
            ]);

            controls.appendChild(details);
        }

        buildSlider(parameter) {
            const fromModel = (value) => parameter.angle ? toDegrees(value) : value;
            const toModel = (value) => parameter.angle ? toRadians(value) : value;

            const decimals = decimalsOf(parameter.step);
            const id = `me-input-${this.config.name}-${parameter.key}`;

            const range = element('input', {
                type: 'range', id: id,
                min: parameter.min, max: parameter.max, step: parameter.step,
            });

            const number = element('input', {
                type: 'number', class: 'me-number', step: parameter.step,
                'aria-label': `${parameter.key} value`,
            });

            const unit = element('span', { class: 'me-unit', text: parameter.angle ? '°' : '' });

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

            return element('div', { class: 'me-slider' }, [
                element('label', { for: id, text: parameter.key }),
                element('div', { class: 'me-slider-inputs' }, [range, number, unit]),
            ]);
        }

        syncInputs() {
            for (const sync of Object.values(this.inputs)) {
                sync();
            }
        }

        update() {
            const data = this.config.compute(this.parameters, this.xValues);

            this.data = data;

            this.config.panels.forEach((panelConfig, index) => {
                this.panels[index].update(
                    this.xValues,
                    data,
                    panelConfig.domain(data, this.parameters),
                    panelConfig.markers(this.parameters)
                );
            });

            this.updateJson();

            if (this.hoverIndex !== undefined) {
                this.showHover(this.hoverIndex);
            } else {
                this.clearReadout();
            }
        }

        updateJson() {
            if (this.json === undefined) {
                return;
            }

            const output = {};

            for (const group of this.config.controlGroups) {
                for (const parameter of group.parameters) {
                    output[parameter.key] = Number(this.parameters[parameter.key].toPrecision(6));
                }
            }

            this.json.textContent = JSON.stringify(output, null, 4);
        }

        onPointerMove(event, panel) {
            const xAxis = this.config.xAxis;

            const point = new DOMPoint(event.clientX, event.clientY)
                .matrixTransform(panel.svg.getScreenCTM().inverse());

            const x = Math.max(xAxis.min, Math.min(xAxis.max, panel.xInverse(point.x)));

            this.hoverIndex = Math.round((x - xAxis.min) / xAxis.step);
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
            const x = this.xValues[index];

            for (const panel of this.panels) {
                panel.showHover(index, x);
            }

            this.readout.replaceChildren(
                ...this.config.readout(x, this.data, index).map(
                    ([name, value]) => this.readoutItem(name, value)
                )
            );
        }

        readoutItem(name, value) {
            return element('span', { class: 'me-readout-item' }, [
                element('span', { class: 'me-readout-name', text: name }),
                element('span', { class: 'me-readout-value', text: value }),
            ]);
        }

        clearReadout() {
            this.readout.replaceChildren(element('span', {
                class: 'me-readout-hint',
                text: this.config.hint,
            }));
        }
    }

    window.StormbirdModelExplorer = { ModelExplorer, niceDomain, toRadians, toDegrees };
})();
