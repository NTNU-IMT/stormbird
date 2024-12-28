let x_trace = {
    x: [],
    y: [],
    type: 'scatter',
    mode: 'lines',
    name: 'Force in x direction',
};

let y_trace = {
    x: [],
    y: [],
    type: 'scatter',
    mode: 'lines',
    name: 'Force in y direction',
};

let layout = {
    title: 'Integrated forces as a function of time',
    xaxis: { title: 'Time' },
    yaxis: { title: 'Value' }
};

let fetch_interval = 500;

let div_name = 'forcePlot';

Plotly.newPlot(div_name, [x_trace, y_trace], layout);

 // Periodically fetch data from server
 setInterval(
    () => {
        fetch('/get-forces')
            .then(response => response.text())
            .then(text => {
                if (text) {
                    let data = JSON.parse(text);

                    x_trace.x = data.time;
                    x_trace.y = data.force_x;

                    y_trace.x = data.time;
                    y_trace.y = data.force_y;
                    
                    Plotly.redraw(div_name);
                }
            });
    }, 
    fetch_interval
);