// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

///
/// Guard that disables floating point exception trapping while calling the Stormbird library.
///
/// OpenFOAM enables trapping of floating point exceptions (FOAM_SIGFPE), so that any division by
/// zero, invalid operation or overflow kills the process. Rust, and the LLVM optimizer, assumes
/// that floating point exceptions are masked. Optimized Rust code can therefore execute floating
/// point operations whose results are never used, for instance a division hoisted out of a branch
/// that is not taken, which is harmless without trapping, but fatal with it.
///
/// The guard saves the floating point environment and masks all exceptions when created. When it
/// goes out of scope, the exception flags raised in the meantime are cleared, and the original
/// environment, including the trapping set by OpenFOAM, is restored.
///

#ifndef FLOATING_POINT_GUARD_H
#define FLOATING_POINT_GUARD_H

#include <cfenv>

class FloatingPointExceptionGuard {
public:
    FloatingPointExceptionGuard() {
        // Saves the environment, clears the flags and masks all exceptions
        std::feholdexcept(&this->saved_environment);
    }

    ~FloatingPointExceptionGuard() {
        // Clear the flags first, so that restoring the environment does not raise exceptions that
        // happened inside the guard
        std::feclearexcept(FE_ALL_EXCEPT);
        std::fesetenv(&this->saved_environment);
    }

    FloatingPointExceptionGuard(const FloatingPointExceptionGuard&) = delete;
    FloatingPointExceptionGuard& operator=(const FloatingPointExceptionGuard&) = delete;

private:
    std::fenv_t saved_environment;
};

#endif
