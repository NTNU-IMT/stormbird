# Stormbird
Stormbird is a library for simulating lifting surfaces, i.e. wings, under the assumption that they 
can be represented as *line-models*. Although this makes it usable for a variety of different cases, 
it is also mostly developed to offer efficient modeling of modern wind propulsion devices. That is, 
the following types of lifting surfaces are of particular interest:

1) Wing sails
2) Rotor sails
3) Suction sails
4) Kites

These use cases require modeling of strong viscous effects on the lift (e.g., due to large angles of 
attack), various lift generation mechanisms (classical foils, with and without flaps, rotating 
cylinders, foils with boundary layer suction), interaction between multiple lifting surfaces 
(when there are multiple sails), interaction between lifting surfaces and other structures (the 
superstructure and other deck structures) and unsteady effects (kites flying dynamically, analysis 
of ships operating in waves, maneuvering simulations).

At the same time, it is also often necessary with efficient computations. The user will usually be 
interested in testing many different weather conditions, ship speeds, sail configurations, and 
operational variables. The goal is, therefore, to find the right balance between accuracy and speed 
for the intended use case. To achieve this, the library supports the following methods, that offer 
different levels of complexity and computational speed:

 1) Discrete static lifting line, for steady- or quasi-steady cases
 2) Discrete dynamic lifting line, for unsteady or steady cases with large wake deformations
 3) Actuator line, for steady and unsteady cases where interaction with other structures is of 
 interest

See the `stormbird` docs or the `book` for more details on the methods

## Folder structure
- `stormbird` contains the core library, written in [Rust](https://www.rust-lang.org/).
- `book` contains a [mdBook](https://github.com/rust-lang/mdBook) that aims to explain the methods 
implemented in Stormbird more thoroughly than what is done in the code docs, together with 
references to papers with more information. **Waring**: This is currently very much a work in 
progress.
- `pystormbird` contains a Python interface to the core library. This can either be used to run 
lifting line simulations directly, for instance using a scripting approach, or to test individual 
parts of the library, for instance when plotting is useful. The interface is generated using the 
[PyO3](https://pyo3.rs/v0.20.3/) crate.
- `cfd_interfaces` contains interfaces to the actuator line functionality for specific CFD solvers. 
At the moment, only [OpenFOAM](https://www.openfoam.com/) is covered, but this might be extended in 
the future. In addition, there is a general C++ interface to the actuator line functionality, as 
OpenFOAM, and many other CFD solvers, are written in C++.
- `fmus` contains a [functional mockup interfaces](https://fmi-standard.org/) for 
running lifting line simulations using the library, as well as other useful FMUs for setting up 
numerical experiments. The interface is generated using the [fmu_from_struct](https://github.com/jarlekramer/fmu_from_struct)
derive macro. This can, for instance, be used to run simulations with
[Open Simulation Platform](https://opensimulationplatform.com/), or load the model in other software that
supports the FMI-standard

## Instructions for how to use the library
Each of the folders contains its own `README` file with install instructions.

For an explanation of how to set up models for different cases, see the `book` (work in progress)

## How to contribute
1) **Contact Jarle Vinje Kramer**, for instance at jarle.a.kramer@ntnu.no, for a discussion about what and how. 
2) Make a **separate branch** where you can implement your modification without affecting the main branch. 
3) If you add new functionality or fix a newly discovered bug, strongly consider **implementing a 
test** for this. 
4) When you think you are done, make sure to run test builds and the actual tests written for the library. 
**They should all pass!**
5) When you are done, make a pull-request. Wait for the pull-request to be **reviewed and approved**.
6) When the pull-request is approved, it should be merged into the main branch using a **squash commit**, 
with a **descriptive commit message**.

### Developing principles
Although there are no absolute rules, if anything, the code loosely follows
[data orientation](https://en.wikipedia.org/wiki/Data-oriented_design). This can mean different 
things to different people, but in this case, the following principles are seen as important:

- It is better to use many arrays with simple data structures rather than a few arrays with complex 
structures. This can make it easier to process things in parallel and avoid computational delays
due to cache misses.
- Internal states are used with care. That is, consider using input variables to methods when 
possible, rather than a bunch of set-method calls to update the internal state. However, internal 
states are still used, because sometimes it is the simplest thing to do...
- Unless there is a significant performance reason (which it may be sometimes), don't represent the 
same data in different ways using multiple variables. Or, in other words, try to limit the data to 
what is necessary. Access to data in different ways for different use cases can still be 
done using method calls that return transformed variations of the same data. This is to avoid 
situations where one version of the data is updated, but another version is forgotten by accident.
- Don't bother about hypothetical future extensions. This library is currently only meant to 
implement three methods - static lifting line, dynamic lifting line, and actuator line - for the 
specific use case of simulating modern sails - wingsails, rotor sails, suction sails and kites. 
Other use cases can be considered in the future, but, if this happens, then it is a future problem, 
not a present problem. The goal is to be **done** at some point. 
- It is very much ok, and often recommended, to move complexity to either the setup phase or the 
post-processing phase if possible. That is, adding some complexity in the setup or post-processing 
steps is seen as acceptable if it reduces the complexity of the library itself. If the setup becomes
very complex, it is possible to add a separate library for this task...

### Code Style
- The code should follow the official [Rust style guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use descriptive names for both functions and variables.
- **Doc strings are good!** Use them! However, explainer comments inside a method or function are generally
a sign of bad code. Can you re-structure or re-name the code such that the explainer comment is not 
needed?


## Developing status
All parts of this repository are work in progress. Breaking changes is therefore expected on all 
fronts.

The core functionality of the three methods is, more or less, in place. That is, it is possible to 
run all methods, and they have gone through some basic testing. 

However, a lot of detailed functionality is missing or in need of improvement. A non-exhaustive 
list is given below to highlight some important limitations:

### Dcoumentation
- The `book` is barely started. Much work is required to get the documentation to a sufficient level
where external users can use the library efficiently. This is acknowledged as a very important 
to-do point!

### Sail types
- Single-element wingsails and rotor sails are both technically implemented. However, the rotor sail 
functionality leads to very unstable simulations and requires care when executing. More work is 
planned on stabilizing the functionality. 
- Suction sails and multi-element wing sails can be tested in a simplified manner by setting 
appropriate foil models for a given suction rate or flap angle. However, in this case, it will not 
be possible to change the parameters during a time-domain simulation. This functionality is planned,
but yet to be implemented.

### Dynamic simulations
- Motion is currently only covered for lifting line simulations. The challenge with actuator line
simulations is mainly connected to the design of the interface (how should the motion be controlled?)
- The motion affects the forces on the wings through an effective velocity, calculated as the 
difference in position for each control point divided by the time step. However, some known effects 
are yet to be implemented:
    - added mass (probably not that important?)
    - dynamic stall (might be important?)
    - effect of rotating chord vectors (they are now rotated along with the rest of the wing, but 
    the rotation velocity does not affect the lift, only their static position/angle of attack)

### Empirical models
- At least for rotor sails, and perhaps also for suction sails, there seems to be a need for some 
empirical modeling to accurately capture lift-induced drag. In other words, the core functionality of
the methods struggles to capture the lift-induced velocities with sufficient accuracy when the lift
coefficient becomes too high. This is not seen as a major problem, as corrections can be calculated 
by using results from CFD simulations, but it also requires a system for applying corrections to a 
Stormbird simulation. This is missing.
- There is a need to model interaction effects with superstructures in some way. For the lifting 
line models, this must happen empirically. There is no functionality to cover this as of yet but 
plans to do so in the future
- Models for the atmospheric boundary layer have not yet been implemented.

## Why the name Stormbird?
It is due to a somewhat bad but deliberate translation. In Norwegian, there is a name for a group of 
birds called "[storm fugler](https://no.wikipedia.org/wiki/Stormfugler)", that directly translates 
into storm birds in English. This group of birds includes albatrosses, petrels and storm petrels, 
which are birds living by and on the sea and that generally have long slender wings with high aspect 
ratios. As the methods in the library are particularly suited for modeling wings with high aspect 
ratios, it seemed fitting.

The English name for the group is "[tubenoses](https://en.wikipedia.org/wiki/Procellariiformes)" or 
just "petrels", but the direct translation of the Norwegian name sounded better. After setting the 
name, it was discovered that stormbird is also a nickname used in English, but for a bird type known 
as [pacific koel](https://en.wikipedia.org/wiki/Pacific_koel). However, petrels are the original 
inspiration.

## License
The software is licensed under the GPL3. See [LICENSE](LICENSE) for more.
