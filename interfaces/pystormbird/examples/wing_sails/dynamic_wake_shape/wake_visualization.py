'''
Live visualization of the wake in a lifting line simulation, using pyvista.

The wake is read from the simulation with the `wake_as_vtk` function after each time step, and
shown in a pyvista window that is updated while the simulation runs. The colors show the strength
of the vortex elements in the wake.

Requires pyvista, which can be installed with `pip install pyvista`.
'''

import numpy as np

from pystormbird.lifting_line import Simulation

def wake_mesh(simulation: Simulation):
    '''
    Returns the current wake of the simulation as a pyvista mesh, with the strength and viscous core
    length of each vortex element as cell data.
    '''
    import pyvista as pv
    from vtkmodules.vtkIOLegacy import vtkPolyDataReader

    data = simulation.wake_as_vtk(binary=True)

    reader = vtkPolyDataReader()
    reader.ReadFromInputStringOn()
    reader.SetBinaryInputString(data, len(data))
    reader.ReadAllScalarsOn()
    reader.Update()

    return pv.wrap(reader.GetOutput())

class LiveWakeVisualization:
    '''
    A pyvista window that shows the wake of a simulation, and is updated each time `update` is
    called. The window is opened at the first update, and closed with `close`.
    '''
    def __init__(self, title: str = "Wake"):
        import pyvista as pv

        self.plotter = pv.Plotter(title=title)
        self.mesh = None
        self.max_strength = 0.0

    def update(self, simulation: Simulation):
        new_mesh = wake_mesh(simulation)

        # A quasi-steady wake is empty before the first time step
        if new_mesh.n_cells == 0:
            return

        # The color range is symmetric around zero, and only grows, so that the colors are
        # comparable between time steps
        self.max_strength = max(
            self.max_strength, float(np.max(np.abs(new_mesh["strength"]))), 1e-12
        )

        color_range = [-self.max_strength, self.max_strength]

        if self.mesh is None:
            self.mesh = new_mesh

            self.plotter.add_mesh(
                self.mesh,
                scalars = "strength",
                cmap = "coolwarm",
                clim = color_range,
                show_edges = True,
                edge_color = "grey",
                line_width = 0.5,
            )

            self.plotter.add_axes()

            # Opens the window without blocking, so that the simulation can continue
            self.plotter.show(interactive_update=True)
        else:
            self.mesh.copy_from(new_mesh)
            self.plotter.update_scalar_bar_range(color_range)

        self.plotter.update()

    def close(self):
        self.plotter.close()
