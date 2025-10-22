var litterature_data = [
    {
        "label": "Bordogna, 2019, experiments",
        "spin_ratios": [0.4649564819056342, 0.9619789280806232, 1.202473660100779, 1.4429683921209346, 1.7361429225836014, 2.0384791571232252, 2.2835547411818604, 2.526339899221256, 2.732478240952818,
        3.192853870819973, 3.650939074667888, 4.125057260650482, 4.596885020613834],
        "lift_coefficients": [1.3987654320987648, 2.8518518518518494, 3.5240740740740737, 4.128395061728394, 5.0586419753086425, 5.7104938271604935, 6.328395061728395, 7.272222222222222, 6.925925925925925,
        7.6728395061728385, 8.019135802469135, 8.596296296296295, 9.519753086419753],
    },
    {
        "label": "Ostman, 2023, CFD",
        "spin_ratios": [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0],
        "lift_coefficients": [0.0, 1.22, 2.56, 5.93, 9.10, 10.77, 12.80, 13.71, 16.90],
        "drag_coefficients": [0.457, 0.411, 0.296, 0.093, 0.066, 0.042, 0.064, 0.05, 0.076]
    }
]

var traces  = [];

for (var i = 0; i < litterature_data.length; i++) {
  var dataset = litterature_data[i];
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
    range: [0, 0.6]
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

Plotly.newPlot('rotor-sail-2d-forces', traces, layout);
