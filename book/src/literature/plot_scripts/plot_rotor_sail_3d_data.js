var literature_data = [
    {
        "label": "Prandtl, 1926, experiments",
        "spin_ratios": [0.0, 0.6795980928695401, 1.4451849610660772, 2.020848433666039, 2.765937639227838, 3.3395978933789054, 3.437015819607268, 4.580969923462093],
        "lift_coefficients": [0.0, 0.48881854996908025, 2.386722568375415, 4.514090688442178, 7.238983129742059, 8.43743973720434, 8.424339852243937, 8.803787662169261]
    },
    {
        "label": "Gerhardt, 2022, CFD",
        "spin_ratios": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        "lift_coefficients": [2.2841530054644714, 5.8360655737704885, 8.754098360655734, 9.61748633879781, 9.63934426229508, 9.846994535519123, 10.12021857923497, 10.316939890710382, 10.338797814207648],
        "drag_coefficients": [0.4375569735642646, 1.143117593436644, 2.74567000911577, 3.54968094804011, 3.7192342752962633, 3.8614402917046498, 3.9708295350957163, 4.266180492251597, 4.53418413855971]
    },
    {
        "label": "Ostman, 2023, CFD",
        "spin_ratios": [0.0, 1.0, 3.0, 5.0],
        "lift_coefficients": [0, 2.62, 8.973, 10.351],
        "drag_coefficients": [0.6459, 0.47, 2.416, 3.381]
    },
    {
        "label": "KSP WIND, 2024, CFD, no foundation",
        "spin_ratios": [0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        "lift_coefficients": [0, 2.38, 5.85, 9.49, 9.96, 11.15],
        "drag_coefficients": [0.5, 0.412,0.950, 1.995, 2.224, 2.775]
    },
    {
        "label": "KSP WIND, 2024, CFD, with foundation",
        "spin_ratios": [0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        "lift_coefficients": [0.00, 1.81, 4.95, 6.86, 7.08, 7.46],
        "drag_coefficients": [0.500, 0.513, 1.374, 2.632, 2.889, 3.153]
    }
]



var traces  = [];

for (var i = 0; i < literature_data.length; i++) {
  var dataset = literature_data[i];
  var color = plotlyColors[i];

  var cl_trace = {
    x: dataset.spin_ratios,
    y: dataset.lift_coefficients,
    mode: 'lines+markers',
    name: dataset.label,
    type: 'scatter',
    xaxis: 'x',
    yaxis: 'y',
    showlegend: true,
    marker: { color: color },
    line: { color: color },
    hovertemplate: 'Value: %{y}<extra></extra>'
  };

  traces.push(cl_trace);

  if (dataset.hasOwnProperty('drag_coefficients')) {
    var cd_trace = {
      x: dataset.spin_ratios,
      y: dataset.drag_coefficients,
      mode: 'lines+markers',
      name: dataset.label,
      type: 'scatter',
      xaxis: 'x2',
      yaxis: 'y2',
      showlegend: false,
      marker: { color: color },
      line: { color: color },
      hovertemplate: 'Value: %{y}<extra></extra>'
    };

    traces.push(cd_trace);
  }
}

var layout = {
  height: 500,
  grid: {
    rows: 1,
    columns: 2,
    pattern: 'independent'
  },
  xaxis: {
    title: 'Spin Ratio',
    domain: [0, 0.45],
    range: [0, 5.1]
  },
  yaxis: {
    title: 'Lift Coefficient (CL)',
    range: [0, 12]
  },
  xaxis2: {
    title: 'Spin Ratio',
    domain: [0.55, 1],
    range: [0, 5.1]
  },
  yaxis2: {
    title: 'Drag Coefficient (CD)',
    anchor: 'x2',
    range: [0, 5]
  },
  showlegend: true,
  legend: {
    x: 0.0,
    y: -0.2,
    orientation: 'h'
  },
  margin: {
    l: 40,
    r: 40,
    b: 40,
    t: 40,
  },
};

Plotly.newPlot('rotor-sail-3d-forces', traces, layout);
