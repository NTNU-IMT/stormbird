# Two moving wings

This example shows how to model two wing sails on a ship that rolls, using the Stormbird lifting line FMU together with a simple FMU that gives a tabulated motion. The sails have no controller, so the local wing angles are given as input to the FMU.

## Running the example
1. Build the FMUs by running `cargo fmu-from-struct --release` in the `stormbird_lifting_line` folder and in the `utils/tabulated_motion` folder. See the README in the `stormbird_lifting_line` folder for more.
2. Run `python run.py`. This writes the parameters file for the Stormbird FMU, `stormbird_parameters.json`, and the motion file, `motion.csv`, and runs the simulation with the OSP command line interface. See the README in the `examples` folder for how to set it up. The parameters file is generated with the [stormbird_setup](https://pypi.org/project/stormbird_setup/) package, which must be installed.
3. The scripts in the `post_process` folder plot the results.

Check the `error_code` output of the Stormbird FMU if the forces are zero. See the README in the `stormbird_lifting_line` folder for the meaning of each code.
