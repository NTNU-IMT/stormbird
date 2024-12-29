function wakePlot() {
    var plotData = {
        type: "mesh3d",
        x: [],
        y: [],
        z: [],
        i: [],
        j: [],
        k: [],
        intensity: [],
        intensitymode: 'cell',
        colorscale: "Viridis"
    };

    let layout = {
        title: 'Shape and strength of wake',
        scene: {
            camera: {
                eye: {x: 1.25, y: 1.25, z: 1.25}
            }
        }
    };

    let divName = 'wakePlot';

    Plotly.newPlot(divName, [plotData], layout);
    let fetchInterval = 1000;

    setInterval(
        () => {
            fetch('/get-wake-shape')
                .then(response => response.text())
                .then(text => {
                    if (text) {
                        let data = JSON.parse(text);

                        plotData.x = data.x;
                        plotData.y = data.y;
                        plotData.z = data.z;
                        plotData.i = data.i;
                        plotData.j = data.j;
                        plotData.k = data.k;
                        plotData.intensity = data.strength;

                        Plotly.react(divName, [plotData], layout);
                    }
                });
        }, 
        fetchInterval
    );
}