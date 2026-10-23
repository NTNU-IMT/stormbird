var literature_data = [
  {
    "label": "C<sub>Q</sub> = 0.047",
    "suction_coefficient": 0.047,
    "angles_of_attack": [1.0988, 3.0705, 5.1843, 7.1582, 9.1784, 11.2907, 13.1302, 14.9704, 17.2711, 19.3415, 21.3681, 23.2567, 25.0531, 27.0804, 29.0174, 31.2770, 33.1161],
    "lift_coefficients": [4.2664, 4.6771, 5.0183, 5.3825, 5.7468, 6.1205, 6.3871, 6.6397, 6.9438, 7.2245, 7.4494, 7.6556, 7.8524, 8.0634, 8.2232, 8.4158, 2.6932],
    "drag_polar_lift_coefficients": [4.2643, 4.7521, 5.0401, 5.3932, 5.7420, 6.1371, 6.4163, 6.9749, 7.4875, 7.8703, 8.0945, 8.2630, 8.4503, 2.6713],
    "drag_coefficients": [0.5942, 0.6381, 0.6617, 0.6946, 0.7464, 0.7942, 0.8424, 0.9462, 1.0652, 1.1921, 1.2706, 1.3454, 1.4314, 1.8008]
  },
  {
    "label": "C<sub>Q</sub> = 0.042",
    "suction_coefficient": 0.042,
    "angles_of_attack": [1.0693, 3.0427, 5.1100, 7.2226, 9.1984, 11.3562, 13.2874, 15.2186, 17.0112, 19.2185, 21.2888, 23.1759, 25.2495, 27.2824, 29.1660, 31.1278],
    "lift_coefficients": [3.9040, 4.2775, 4.6233, 4.9923, 5.3147, 5.7070, 5.9923, 6.2775, 6.5579, 6.8806, 7.1613, 7.4000, 7.6110, 7.7012, 7.0153, 2.6402],
    "drag_polar_lift_coefficients": [3.8982, 4.3348, 4.6321, 4.9759, 5.3014, 5.6826, 5.9986, 6.3242, 6.6035, 6.9014, 7.1623, 7.4047, 7.5774, 7.7040, 7.6127, 7.0304, 2.6320],
    "drag_coefficients": [0.6046, 0.6373, 0.6666, 0.6977, 0.7400, 0.7860, 0.8209, 0.8727, 0.9209, 0.9765, 1.0398, 1.0993, 1.1553, 1.2227, 1.2907, 1.3807, 1.6956]
  },
  {
    "label": "C<sub>Q</sub> = 0.032",
    "suction_coefficient": 0.032,
    "angles_of_attack": [1.2699, 3.1045, 5.2177, 7.3317, 9.2610, 11.3768, 13.3987, 15.2389, 17.2646, 19.4777, 21.3441, 22.8366, 23.3331],
    "lift_coefficients": [3.5696, 3.9430, 4.2981, 4.6346, 4.9617, 5.2610, 5.5881, 5.8407, 6.0842, 6.2814, 5.9672, 3.7291, 3.0047],
    "drag_polar_lift_coefficients": [3.6204, 4.0014, 4.3452, 4.6890, 5.2744, 5.5810, 5.8041, 6.1018, 6.2601, 5.9376, 2.9763],
    "drag_coefficients": [0.6242, 0.6608, 0.6900, 0.7229, 0.7815, 0.8070, 0.8328, 0.8771, 0.9105, 1.0093, 1.3108]
  },
  {
    "label": "C<sub>Q</sub> = 0.026",
    "suction_coefficient": 0.026,
    "angles_of_attack": [1.1002, 3.1200, 5.1406, 7.1642, 9.0018, 11.0275, 13.2406, 17.3459, 18.7623, 19.6137],
    "lift_coefficients": [3.2349, 3.6085, 3.9635, 4.2535, 4.5618, 4.8053, 5.0025, 4.3280, 3.7346, 3.3451],
    "drag_polar_lift_coefficients": [3.2778, 3.6495, 4.2721, 4.5693, 5.0053, 4.7755, 4.4615, 4.2878, 3.3070, 2.9477],
    "drag_coefficients": [0.6477, 0.6843, 0.7428, 0.7646, 0.7747, 0.8694, 0.9305, 1.0514, 1.1594, 1.2733]
  },
  {
    "label": "C<sub>Q</sub> = 0.023",
    "suction_coefficient": 0.023,
    "angles_of_attack": [1.1269, 3.1510, 5.3593, 6.8287, 9.2196, 11.0654, 13.1962, 15.5081],
    "lift_coefficients": [2.6588, 2.9395, 3.2389, 3.5005, 3.8558, 3.9876, 3.9617, 3.0253],
    "drag_polar_lift_coefficients": [2.6480, 2.9632, 3.2239, 3.5218, 3.8386, 4.0068, 3.9339, 3.0357, 2.8129],
    "drag_coefficients": [0.6833, 0.6824, 0.7306, 0.7863, 0.8569, 0.9167, 0.9808, 1.0435, 1.0328]
  },
  {
    "label": "C<sub>Q</sub> = 0.018",
    "suction_coefficient": 0.018,
    "angles_of_attack": [1.1424, 3.1214, 5.2844, 7.2636, 9.3419, 11.2858, 13.0999, 15.3825],
    "lift_coefficients": [2.3243, 2.5770, 2.8578, 3.1059, 3.2147, 3.2258, 3.0416, 2.7371],
    "drag_polar_lift_coefficients": [2.3260, 2.6139, 2.8837, 3.1164, 3.2564, 3.0625, 2.7467, 2.5616],
    "drag_coefficients": [0.8083, 0.8301, 0.8708, 0.9116, 0.9563, 0.9945, 0.9728, 0.9883]
  },
  {
    "label": "C<sub>Q</sub> = 0.0",
    "suction_coefficient": 0.0,
    "angles_of_attack": [1.1609, 3.1389, 5.3963, 7.3308, 9.3621, 11.5350, 13.3404, 15.2051],
    "lift_coefficients": [1.9247, 2.2007, 2.4398, 2.6553, 2.7780, 2.8450, 2.8467, 2.5697],
    "drag_polar_lift_coefficients": [1.9793, 2.2304, 2.4538, 2.6122, 2.7885, 2.8912, 2.6505],
    "drag_coefficients": [0.6362, 0.6731, 0.7102, 0.7474, 0.7544, 0.7842, 0.8037]
  }
]

var traces  = [];

for (var i = 0; i < literature_data.length; i++) {
  var dataset = literature_data[i];
  var color = plotlyColors[i];

  var cl_trace = {
    x: dataset.angles_of_attack,
    y: dataset.lift_coefficients,
    mode: 'lines',
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
      x: dataset.drag_polar_lift_coefficients,
      y: dataset.drag_coefficients,
      mode: 'lines',
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
    title: 'Lift Coefficient (CL)',
    range: [0, 9]
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
