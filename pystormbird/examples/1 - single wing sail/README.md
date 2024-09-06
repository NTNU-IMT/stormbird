# Single wing sail

The point of this example is to demonstrate how a single wing sail can be simulated using the lifting line functionality in Stormbird. 

In particular, the example aims to demonstrate best practices to get stable and accurate results also when the angle of attack and lift coefficient is high. 

Two cases are used to demo this:
- A single element wing sail, where both experimental and CFD data is available. 
- A two-element wing sails, where only CFD data is available.

## File structure
- `graf_2014_data.json`: contains both experimental and CFD data for a rectangular single element sail.