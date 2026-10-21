var literature_data = [
  {
    "label": "Cousteau, 1985, experiments",
    "power_coefficients": [0.0422, 0.0459, 0.0589, 0.0812, 0.1128, 0.1519, 0.1966, 0.2515, 0.3157, 0.3846, 0.4516, 0.5316, 0.6052, 0.6722, 0.7290],
    "lift_coefficients": [2.5083, 2.9675, 3.6383, 4.2266, 4.8060, 5.3673, 5.8735, 6.2607, 6.5379, 6.7234, 6.7986, 6.8466, 6.8577, 6.8778, 6.8793]
  }
]

var traces  = [];

for (var i = 0; i < literature_data.length; i++) {
  var dataset = literature_data[i];
  var color = plotlyColors[i];

  var cl_trace = {
    x: dataset.power_coefficients,
    y: dataset.lift_coefficients,
    mode: 'lines',
    name: dataset.label,
    type: 'scatter',
    showlegend: false,
    marker: { color: color },
    line: { color: color },
    hovertemplate: 'Value: %{y}<extra></extra>'
  };

  traces.push(cl_trace);
}

var layout = {
  height: 500,
  xaxis: {
    title: 'Power Coefficient (CP)',
    range: [0, 0.8]
  },
  yaxis: {
    title: 'Lift Coefficient (CL)',
    range: [0, 8]
  },
  margin: {
    l: 40,
    r: 40,
    b: 40,
    t: 40,
  },
};

Plotly.newPlot('suction-sail-power', traces, layout);
