# Tutorials

This section gives an introduction to the tutorials made for the library. At the moment, only the lifting line functionality is covered for a limited set of use cases. However, the plan is to extent this section in the future to cover more use cases and also the actuator line functionality. 

This text is only meant as a high-level overview. To get the full picture, it is necessary to look at the detailed code and input files for each tutorial. These files should be distributed along with this book. At the time of writing this (May 2024), the code repository is not yet open. However, this book, along with code documentation and tutorial files is distributed using a Teams-site for project partner in KSP WIND. On that site, there should be a folder named `examples` with sub-folders for each example.

## FMU vs Python
The current tutorials focuses on how to run simulations using the Python interface to Stormbird. However, running FMU simulations are very similar in nature, and the setup is more or less identical. The main difference is connected to how FMUs work relative to Python code. 

## Setup and post-processing
As mention in [input/output-chapter](io_info.md), Stormbird generally uses JSON strings to both serialize output and deserialize input. These JSON strings are the same no matter which version of Stormbird is used. 

Although these strings can easily be generated manually, for instance in a file, it is often practical to have a procedural setup process where variables in the input can be adjusted based on some logic. As such, the examples will generally show how to generate the setup using a scripting approach based on Python. 

Python has excellent support for serializing dictionaries into JSON strings, which makes the conversion from Python variables into a Stormbird-friendly input string straight forward. For the same reason, the implementation of pystormbird is also kept simple by relying on JSON strings as input, rather than a full Python class implementation of the input structures. 

As Python is used for both the setup and execution of tutorials, it is also used for post-processing. The Stormbird results are parsed into Python dictionaries from JSON strings, which can further be used for plotting or other post-processing tasks. 

However, both the setup and the post-processing is generally independent of the pystormbird implementation, which means both steps could easily be changed to other programming languages if wanted. This could for instance be an option if the FMU version of Stormbird is used and some other scripting language is preferred. Python is just used as a convenient example.