# External data sources

Although Stormbird is developed to capture the fundamental physics of lifting surfaces, it is still necessary to use some amount of external data to properly tune the models. In particular, the [sectional models](./../sectional_models/sectional_models_intro.md) that are used to compute two-dimensional lift and drag are entirely dependent on empirical data to give accurate results. In addition, empirical data for three-dimensional cases are also interesting for validation purposes. 

The following sections in this chapter gives references and links to useful data sources that can be used to either set up or validate a Stormbird model [^note_on_future_plans]

The references are sorted into different sections based on sail type, and under different subheadings based on the source of the data. The following categories are deemed interesting:
- **Experimental data**, meaning measurements done in a controlled lab experiment.
- **Full scale measurements**, meaning measurements from actual operation of wind propulsion devices
- **CFD data**, meaning simulations using high-fidelity methods, such as the finite volume method.

[^note_on_future_plans]: although not ready at the moment, it is also planned to implement standardized test scripts for all external data sources, to show how they can be reproduced using Stormbird.   