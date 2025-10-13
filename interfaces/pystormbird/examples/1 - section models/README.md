# Examples of section models

This folder contains examples of different ways to set up section models to be used in Stormbird. 

Below is a short description of each example. Please read the Python files for more details.

## Tuning a foil model to fit CFD data
The file `naca_0012_Graf2014.py` uses the general functionality un `foil_tuner.py` to fit a foil model to CFD data extracted from a paper (link in the Python file). 

This is achieved using the optimization functionality in Sci-Py, together with the Python interface to the `Foil` section model in Stormbird. 

## Manual set up of multi-element foil model
The file `manual_multi_element_foil.py` shows how a multi-element foil section can be set up manually. This demonstrates the basic data structure of the `VaryingFoil` section model.