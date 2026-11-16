"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Plotting utilities for the reports of the standard simulations. Requires plotly, which is an
optional dependency (install with the "report" extra).
"""

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
) -> None:
    '''
    Adds a dropdown menu that shows one group of traces at a time. The traces must already be in
    the figure, and trace_group_index gives the index of the group that each trace belongs to, in
    the same order as the traces. The labels and titles are given for each group. The first group
    is shown initially. The title of the figure is replaced when a group is selected, while the
    other title settings, e.g., the position, are kept.

    Optionally, the layout shapes, e.g., vertical lines, can be given for each group. They replace
    all shapes in the figure when a group is selected.

    The menu is placed at (x, y) in paper coordinates, anchored to the left. Make sure that there is
    room for it in the margin, so that it does not cover the title or the subplot titles.
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
        x=x, y=y, yanchor=yanchor,
    )


def add_selector(
    figure: Any,
    labels: list[str],
    visible: list[list[bool]],
    layout_updates: list[dict[str, Any]] | None = None,
    x: float = 0.0,
    y: float = 1.12,
    yanchor: str = "bottom",
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
        )
    )

    figure.update_layout(updatemenus=menus)


def write_html(figures: list[Any], file_path: Path | str) -> None:
    '''
    Writes several plotly figures to the same HTML file. The plotly library is embedded once, with
    the first figure, so that the file also works offline.
    '''
    parts = [
        figure.to_html(full_html=False, include_plotlyjs=(index == 0))
        for index, figure in enumerate(figures)
    ]

    html = (
        "<html>\n<head><meta charset=\"utf-8\" /></head>\n<body>\n"
        + "\n".join(parts)
        + "\n</body>\n</html>\n"
    )

    Path(file_path).write_text(html, encoding="utf-8")
