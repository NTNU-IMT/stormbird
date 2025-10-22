var literature_data = [
  {
      "label": "Cousteau, 1985, experiments, suction coefficient 0.047",
      "suction_coefficient": 0.047,
      "angles_of_attack": [1.158614863360441, 3.0485016660877733, 5.132848074933868, 6.930212615446074, 8.869923012622571, 10.953251574799637, 12.99362700754353, 17.026589971920174,
      19.066456481329524, 21.061334167967615, 24.816221422364407, 26.716541153449313, 28.618387654537784, 30.71189468340519, 32.64025609022197],
      "lift_coefficients": [4.280351233439316, 4.689058021077061, 5.03560427648278, 5.386736567286662, 5.757140512460354, 6.122855365260641, 6.397485028111654, 6.9467100014886025,
      7.230923963036899, 7.462389929425058, 7.8484757109977075, 8.060704375341139, 8.24418014359272, 8.418209022447337, 2.7023428285704485],
      "drag_coefficients": [0.6031423599844823, 0.6453513787880354, 0.6745806690481064, 0.7169446276389406, 0.7611688697796453, 0.8053508556147282, 0.8517593463150686, 0.952101974526486, 1.0098130208781306, 1.0714559158576291,
      1.1912183348097827, 1.2698432341366495, 1.350567869411913, 1.4331809591074611, 1.8032103725144832]
  }
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
    range: [0, 9]
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

Plotly.newPlot('suction-sail-2d-forces', traces, layout);
