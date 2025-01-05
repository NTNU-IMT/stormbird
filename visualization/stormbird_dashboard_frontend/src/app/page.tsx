import React from 'react';
import ClearData from './components/ClearData';
import ForcePlot from './components/ForcePlot';
import CirculationDistributionPlot from './components/CirculationDistributionPlot';
import AverageAnglesOfAttack from './components/AverageAnglesOfAttack';

import './styles.css';

export default function Page() {
  return (
    <div>
      <h1>Stormbird Dashboard</h1>
      <p>
        This page shows output from a Stormbird simualtion using the lifting line FMU
      </p>
      <ClearData />
      <div className="plot-grid">
        <ForcePlot />
        <CirculationDistributionPlot />
        <AverageAnglesOfAttack />
      </div>
    </div>
  );
}