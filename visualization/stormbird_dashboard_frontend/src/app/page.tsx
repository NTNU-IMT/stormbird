'use client';

import React, { useState } from 'react';
import ClearData from './components/ClearData';
import ForcePlot from './components/ForcePlot';
import CirculationDistributionPlot from './components/CirculationDistributionPlot';
import AverageAnglesOfAttack from './components/AverageAnglesOfAttack';
import ServerAddressInput from './components/ServerAddressInput';

import './styles.css';

export default function Page() {
  const [serverAddress, setServerAddress] = useState('');

  return (
    <div>
      <h1>Stormbird Dashboard</h1>
      <p>
        This page shows output from a Stormbird simualtion using the lifting line FMU
      </p>
      
      <ServerAddressInput onAddressChange={setServerAddress} defaultAddress="localhost:8080" />
      <ClearData serverAddress={serverAddress}/>
      <div className="plot-grid">
        <ForcePlot serverAddress={serverAddress} />
        <CirculationDistributionPlot serverAddress={serverAddress} />
        <AverageAnglesOfAttack serverAddress={serverAddress} />
      </div>
    </div>
  );
}