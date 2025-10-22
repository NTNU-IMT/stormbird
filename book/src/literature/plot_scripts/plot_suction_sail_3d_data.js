var literature_data = [
    {
        "label": "Cousteau, 1985, experiments, suction coefficient 0.05",
        "suction_coefficient": 0.05,
        "angles_of_attack": [0.0, 2.6021575308572054, 6.159849566655858, 9.805991884516963, 13.760126680710869, 18.022253955237616, 22.768369410001554, 28.350916879921854, 35.03298505563494, 37.486066535650124],
        "lift_coefficients": [2.63613652108753, 2.999309335633191, 3.5030680484666847, 4.00243747260671, 4.502187928572436, 5.002319416363867, 5.503049668452825, 6.000315994853594, 6.503441304132672, 6.61444527704336],
        "drag_coefficients": [0.4689177335114607, 0.4973690905324748, 0.6013515298199845, 0.7357871806242624, 0.8963637147190511, 1.0744119899962836, 1.2829579624457637, 1.5088867047667716, 1.7478858887330215, 1.8166607120763203]
    },
]

var traces  = [];

for (var i = 0; i < literature_data.length; i++) {
  var dataset = literature_data[i];
  var color = plotlyColors[i];

  var cl_trace = {
    x: dataset.angles_of_attack,
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
      x: dataset.angles_of_attack,
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
    title: 'Angle of attack [deg]',
    range: [0, 45.0]
  },
  yaxis: {
    title: 'Lift Coefficient (CL)',
    range: [0, 8]
  },
  xaxis2: {
    title: 'Angle of attack [deg]',
    range: [0, 45.0]
  },
  yaxis2: {
    title: 'Drag Coefficient (CD)',
    anchor: 'x2',
    range: [0, 2.5]
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

Plotly.newPlot('suction-sail-3d-forces', traces, layout);
