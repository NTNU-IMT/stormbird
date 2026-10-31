'''
Plots the results from the OpenFOAM examples in this folder. This is a thin wrapper around
plotting.py in interfaces/pystormflow/examples/rotor_sails/deybach_wind_tunnel, with OpenFOAM as the
default solver. It has the same commands and options:

    python plot_results.py means
    python plot_results.py time-series --spin-ratio 3.0
    python plot_results.py mesh-convergence --spin-ratio 3.0
    python plot_results.py interference

Use --solver stormflow to plot the Stormflow results instead. The file is not called plotting.py, as
it would then hide the module it imports.
'''

import sys
from pathlib import Path

sys.path.append(str(
    Path(__file__).resolve().parents[5] / "pystormflow" / "examples" / "rotor_sails" /
    "deybach_wind_tunnel"
))

from plotting import Solver, main

if __name__ == '__main__':
    main(Solver.OpenFOAM)
