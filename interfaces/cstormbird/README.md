# A C-interface for the Stormbird library

This crate implements a simple C-interface to the main structures and methods in the Stormbird library. The available functionality is intended to be very similar to what is available in the pystormbird crate, but implement to be used through a C-ABI. This mainly so that library can be interfaced by other languages without their dedicated stormbird interface. In other words, as most languages understand the C-ABI, a general C interface might be useful.
