"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from abc import ABC, abstractmethod

import numpy as np

from .discretized_spectrum import DiscretizedSpectrum

class GustSpectrum(ABC):
    @abstractmethod
    def spectrum_value(self, frequency: float) -> float: ...
    
    def discretize(self, min_freq: float, max_freq: float, nr_freqs: int) -> DiscretizedSpectrum:
        frequencies = np.linspace(min_freq, max_freq, nr_freqs)
        
        delta_freq = frequencies[1] - frequencies[0]
        
        amplitudes = np.zeros(nr_freqs)
        
        phases = np.zeros(nr_freqs)
        
        for i in range(nr_freqs):
            s = self.spectrum_value(frequencies[i])
            
            amplitudes[i] = np.sqrt(2.0 * s * delta_freq)
            
            phases[i] = np.random.rand() * 2 * np.pi

        return DiscretizedSpectrum(
            frequencies=frequencies.tolist(),
            amplitudes=amplitudes.tolist(),
            phases=phases.tolist()
        )
