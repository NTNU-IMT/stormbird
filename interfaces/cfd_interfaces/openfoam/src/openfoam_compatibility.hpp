// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

///
/// Small helpers that hide the differences between the two main OpenFOAM distributions:
///
/// - ESI/Keysight OpenFOAM (openfoam.com), with versions named like v2412
/// - The OpenFOAM Foundation version (openfoam.org), with versions named like 14
///
/// The build system of the ESI version defines the macro OPENFOAM (e.g., OPENFOAM=2412), while the
/// Foundation version does not. This is used to select the right code at compile time, so that the
/// version is chosen automatically from the OpenFOAM environment loaded when building.
///

#ifndef OPENFOAM_COMPATIBILITY_H
#define OPENFOAM_COMPATIBILITY_H

#if defined(OPENFOAM)
    #define STORMBIRD_OPENFOAM_ESI
#else
    #define STORMBIRD_OPENFOAM_FOUNDATION
#endif

#include "fvMesh.H"
#include "Time.H"

#ifdef STORMBIRD_OPENFOAM_FOUNDATION
    #include "meshSearch.H"
#endif

namespace openfoam_compatibility {
    /// The name of the current time, used as the instance of fields
    inline Foam::word time_name(const Foam::Time& time) {
        #ifdef STORMBIRD_OPENFOAM_ESI
            return time.timeName();
        #else
            return time.name();
        #endif
    }

    /// Returns the index of the cell containing the point, or -1 if the point is not in the mesh
    inline Foam::label find_cell(const Foam::fvMesh& mesh, const Foam::point& point) {
        #ifdef STORMBIRD_OPENFOAM_ESI
            return mesh.findCell(point);
        #else
            return Foam::meshSearch::New(mesh).findCell(point);
        #endif
    }
}

#endif
