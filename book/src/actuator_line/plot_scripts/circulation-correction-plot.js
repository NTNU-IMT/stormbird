

var dx = 0.001;
var x_min = -0.5;
var x_max = 0.5;

var exp_factors = [8.0, 16.0, 32.0]

var traces = []
for (var i_exp = 0; i_exp < exp_factors.length; i_exp++) {
  var exp_factor = exp_factors[i_exp];

  var x = [];
  var y = [];

  for (var i = 0; i <= (x_max - x_min) / dx; i++) {
    x.push(x_min + i * dx);

    var s = 0.5 - Math.abs(x[i]);

    var exp_value = Math.exp(-exp_factor * s);

    var y_value = (2.0 / Math.PI) * Math.acos(exp_value);

    y.push(y_value);
  }

  var trace = {
    x: x,
    y: y,
    mode: 'lines',
    name: '\u03B2 = ' + exp_factor.toString(),
  };

  traces.push(trace);
}

var layout = {
  xaxis: {
    title: 'Non-dimensional span distance along the wing',
  },
  yaxis: {
    title: 'Correction factor',
  },
};

Plotly.newPlot('circulation-correction-plot', traces, layout);
