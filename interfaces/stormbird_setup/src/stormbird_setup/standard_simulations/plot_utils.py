"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Plotting utilities for the reports of the standard simulations. Requires plotly, which is an
optional dependency (install with the "report" extra).
"""

import html
from pathlib import Path
from typing import Any

import numpy as np


def averaging_start_index(time: np.ndarray, averaging_fraction: float) -> int:
    '''
    The first index of the time series that is included in the mean over the last part of the
    simulated time. At least one value is always included.
    '''
    end_time = time[-1]
    start_time = end_time - averaging_fraction * (end_time - time[0])

    return min(int(np.searchsorted(time, start_time)), len(time) - 1)


def add_dropdown(
    figure: Any,
    labels: list[str],
    titles: list[str],
    trace_group_index: list[int],
    shapes: list[list[dict[str, Any]]] | None = None,
    x: float = 0.0,
    y: float = 1.12,
    yanchor: str = "bottom",
    pad: dict[str, int] | None = None,
) -> None:
    '''
    Adds a dropdown menu that shows one group of traces at a time. The traces must already be in
    the figure, and trace_group_index gives the index of the group that each trace belongs to, in
    the same order as the traces. The labels and titles are given for each group. The first group
    is shown initially. The title of the figure is replaced when a group is selected, while the
    other title settings, e.g., the position, are kept.

    Optionally, the layout shapes, e.g., vertical lines, can be given for each group. They replace
    all shapes in the figure when a group is selected.

    The menu is placed at (x, y) in paper coordinates, anchored to the left, with an optional
    padding in pixels, e.g., dict(b=50) to place it 50 pixels above y when yanchor is "bottom".
    Make sure that there is room for it in the margin, so that it does not cover the title or the
    subplot titles.
    '''
    if len(trace_group_index) != len(figure.data):
        raise ValueError(
            f"Got {len(trace_group_index)} group indices for {len(figure.data)} traces"
        )

    layout_updates: list[dict[str, Any]] = []

    for group_index, title in enumerate(titles):
        layout_update: dict[str, Any] = {"title.text": title}

        if shapes is not None:
            layout_update["shapes"] = shapes[group_index]

        layout_updates.append(layout_update)

    if shapes is not None:
        figure.update_layout(shapes=shapes[0])

    figure.update_layout(title_text=titles[0])

    add_selector(
        figure,
        labels,
        [[index == group_index for index in trace_group_index] for group_index in range(len(labels))],
        layout_updates,
        x=x, y=y, yanchor=yanchor, pad=pad,
    )


def add_selector(
    figure: Any,
    labels: list[str],
    visible: list[list[bool]],
    layout_updates: list[dict[str, Any]] | None = None,
    x: float = 0.0,
    y: float = 1.12,
    yanchor: str = "bottom",
    pad: dict[str, int] | None = None,
) -> None:
    '''
    Adds a dropdown menu with one option for each label. Each option shows the traces that are true
    in the corresponding list in visible, which has one value for each trace in the figure, and
    optionally applies a layout update. The first option is selected initially. The menu is added
    after any menus already in the figure, and is placed as in add_dropdown.
    '''
    for option_visible in visible:
        if len(option_visible) != len(figure.data):
            raise ValueError(
                f"Got {len(option_visible)} visibility values for {len(figure.data)} traces"
            )

    for trace, trace_visible in zip(figure.data, visible[0]):
        trace.visible = trace_visible

    buttons = []

    for index, (label, option_visible) in enumerate(zip(labels, visible)):
        args: list[Any] = [{"visible": option_visible}]

        if layout_updates is not None:
            args.append(layout_updates[index])

        buttons.append(dict(label=label, method="update", args=args))

    menus = list(figure.layout.updatemenus or [])

    menus.append(
        dict(
            type="dropdown",
            buttons=buttons,
            active=0,
            direction="down",
            showactive=True,
            x=x,
            xanchor="left",
            y=y,
            yanchor=yanchor,
            pad=pad or {},
        )
    )

    figure.update_layout(updatemenus=menus)


def add_data_toggle(
    figure: Any,
    labels: list[str],
    hidden_traces: list[list[int]],
    x: float = 1.0,
    y: float = 1.12,
    xanchor: str = "right",
    yanchor: str = "bottom",
    keep_legend: bool = True,
    pad: dict[str, int] | None = None,
) -> None:
    '''
    Adds a row of buttons, one for each label, where each option hides the data of the traces with
    the indices in the corresponding list in hidden_traces, and shows the data of all other traces
    that are hidden by any of the options. The first option is selected initially.

    The data is hidden by replacing x and y, rather than by changing the visibility, so that the
    buttons can be combined with menus that change the visibility, such as add_selector. If
    keep_legend is True, the data is replaced with a single missing point, so that the hidden traces
    keep their legend entries. Otherwise, it is replaced with empty lists, which also removes the
    legend entries of the hidden traces. The buttons are placed as the menu in add_dropdown.
    '''
    toggled = sorted(set(index for indices in hidden_traces for index in indices))

    if not toggled:
        return

    hidden_data: list[Any] = [None] if keep_legend else []

    original = {index: (list(figure.data[index].x), list(figure.data[index].y)) for index in toggled}

    def data(hidden: set[int]) -> dict[str, list[list[Any]]]:
        return {
            "x": [hidden_data if index in hidden else original[index][0] for index in toggled],
            "y": [hidden_data if index in hidden else original[index][1] for index in toggled],
        }

    for index in hidden_traces[0]:
        figure.data[index].x = hidden_data
        figure.data[index].y = hidden_data

    buttons = [
        dict(label=label, method="restyle", args=[data(set(hidden)), toggled])
        for label, hidden in zip(labels, hidden_traces)
    ]

    menus = list(figure.layout.updatemenus or [])

    menus.append(
        dict(
            type="buttons",
            direction="right",
            buttons=buttons,
            active=0,
            showactive=True,
            x=x,
            xanchor=xanchor,
            y=y,
            yanchor=yanchor,
            pad=pad or {},
        )
    )

    figure.update_layout(updatemenus=menus)


_ASPECT_RATIO_META_KEY = "subplot_width_to_height"


def lock_subplot_aspect_ratio(figure: Any, width_to_height: float) -> None:
    '''
    Makes the height of the figure follow its width in the HTML files written by write_html and
    write_html_with_selector, so that the first subplot, and therefore all subplots of the same size,
    get the given aspect ratio, as the width divided by the height. The height is adjusted when the
    figure is drawn, e.g., when the window is resized. The ratio is stored in the meta data of the
    layout. The title, the menus and the margins should be placed in pixels from the top, so that
    they do not move when the height changes.
    '''
    meta = dict(figure.layout.meta) if isinstance(figure.layout.meta, dict) else {}

    meta[_ASPECT_RATIO_META_KEY] = width_to_height

    figure.update_layout(meta=meta)


# Adjusts the height of the figures with an aspect ratio in the meta data of the layout, see
# lock_subplot_aspect_ratio, each time they are drawn. Hidden figures are skipped, and adjusted when
# they are shown.
_ASPECT_RATIO_SCRIPT = """
<script>
(function () {
    function lockAspectRatio(div) {
        var layout = div._fullLayout;
        var meta = div.layout && div.layout.meta;
        if (!layout || !meta || !meta.%(key)s || div.offsetParent === null) {
            return;
        }
        var xaxis = layout.xaxis;
        var yaxis = layout.yaxis;
        if (!xaxis || !yaxis || !(xaxis._length > 0) || !(yaxis._length > 0)) {
            return;
        }
        var target = xaxis._length / meta.%(key)s;
        if (Math.abs(target - yaxis._length) < 2) {
            return;
        }
        var plotHeight = layout._size.h;
        var height = Math.round(layout.height - plotHeight + plotHeight * target / yaxis._length);
        // The element must get the new height as well, as the figures otherwise overlap
        div.style.height = height + "px";
        Plotly.relayout(div, {height: height});
    }
    window.addEventListener("load", function () {
        document.querySelectorAll(".plotly-graph-div").forEach(function (div) {
            if (div.on) {
                div.on("plotly_afterplot", function () {
                    setTimeout(function () { lockAspectRatio(div); }, 0);
                });
            }
            lockAspectRatio(div);
        });
    });
})();
</script>
""" % {"key": _ASPECT_RATIO_META_KEY}


def write_html(
    figures: list[Any],
    file_path: Path | str,
    title: str | None = None,
    max_width: int | None = None,
) -> None:
    '''
    Writes several plotly figures to the same HTML file. The plotly library is embedded once, with
    the first figure, so that the file also works offline. The optional title is shown in a bar at
    the top of the page, which stays at the top of the window when scrolling. The figures are
    centered, and are at most max_width pixels wide, if given.
    '''
    parts = [] if title is None else [_top_bar(title, "")]

    parts.append(_figures_start(max_width))

    parts += [
        figure.to_html(full_html=False, include_plotlyjs=(index == 0))
        for index, figure in enumerate(figures)
    ]

    parts.append("</div>")

    _write_html_body("\n".join(parts), file_path, title)


# Shows the group of figures with the chosen index and hides the others. The figures are resized
# when shown, as figures that are drawn while hidden do not get the width of the page.
_SHOW_GROUP_SCRIPT = """
<script>
function showFigureGroup(index) {
    document.querySelectorAll(".figure-group").forEach(function (group) {
        var shown = group.dataset.index === index;
        group.style.display = shown ? "" : "none";
        if (shown) {
            group.querySelectorAll(".plotly-graph-div").forEach(function (div) {
                Plotly.Plots.resize(div);
            });
        }
    });
}
</script>
"""


def write_html_with_selector(
    groups: list[tuple[str, list[Any]]],
    file_path: Path | str,
    selector_label: str = "Choose",
    title: str | None = None,
    max_width: int | None = None,
) -> None:
    '''
    Writes groups of plotly figures to the same HTML file, with a single dropdown menu at the top of
    the page that chooses the group that is shown. Each group is given as a label for the menu and
    a list of figures. The first group is shown initially. The menu is in a bar at the top of the
    page, together with the optional title, which stays at the top of the window when scrolling.
    The plotly library is embedded once, so that the file also works offline. The figures are
    centered, and are at most max_width pixels wide, if given.
    '''
    options = "".join(
        f'<option value="{index}">{html.escape(label)}</option>'
        for index, (label, _) in enumerate(groups)
    )

    selector = (
        f'<span><label for="figure-group-selector"><b>{html.escape(selector_label)}:</b></label> '
        '<select id="figure-group-selector" onchange="showFigureGroup(this.value)" '
        f'style="font-size: 14px">{options}</select></span>'
    )

    parts = [_top_bar(title, selector), _SHOW_GROUP_SCRIPT, _figures_start(max_width)]

    include_plotlyjs = True

    for index, (_, figures) in enumerate(groups):
        style = "" if index == 0 else ' style="display: none"'

        parts.append(f'<div class="figure-group" data-index="{index}"{style}>')

        for figure in figures:
            parts.append(figure.to_html(full_html=False, include_plotlyjs=include_plotlyjs))

            include_plotlyjs = False

        parts.append("</div>")

    parts.append("</div>")

    _write_html_body("\n".join(parts), file_path, title)


def _figures_start(max_width: int | None) -> str:
    '''The start of the element that contains the figures, centered with an optional max width'''
    if max_width is None:
        return "<div>"

    return f'<div style="max-width: {max_width}px; margin: 0 auto">'


def _top_bar(title: str | None, content: str) -> str:
    '''A bar at the top of the page, which stays at the top of the window when scrolling'''
    title_html = "" if title is None else (
        f'<span style="font-size: 20px; font-weight: bold">{html.escape(title)}</span>'
    )

    return (
        '<div style="position: sticky; top: 0; z-index: 10; background: white; padding: 10px; '
        'font-family: sans-serif; border-bottom: 1px solid lightgray; display: flex; '
        f'align-items: center; gap: 40px">{title_html}{content}</div>'
    )


def _write_html_body(body: str, file_path: Path | str, title: str | None = None) -> None:
    title_html = "" if title is None else f"<title>{html.escape(title)}</title>"

    html_text = (
        f"<html>\n<head><meta charset=\"utf-8\" />{title_html}</head>\n<body>\n"
        + body
        + _ASPECT_RATIO_SCRIPT
        + "\n</body>\n</html>\n"
    )

    Path(file_path).write_text(html_text, encoding="utf-8")
