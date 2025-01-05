'use client';

import React, { useEffect, useState } from 'react';

const AverageAnglesOfAttack = () => {
    const [data, setData] = useState([]);
    
    const [layout, setLayout] = useState(
        {
            title: 'Average angle of attack on each wing as a function of time', 
            xaxis: { title: 'Time step' }, 
            yaxis: { title: 'Value [deg]' },
            width: 600
        }
    );
    
    const [Plot, setPlot] = useState(null);

    useEffect(() => {
        import('react-plotly.js').then((module) => {
            setPlot(() => module.default);
        });

        const fetchInterval = 500;

        const intervalId = setInterval(() => {
            fetch('http://localhost:8080/get-average-angles-of-attack')
                .then(response => response.text())
                .then(text => {
                    if (text) {
                        const fetchedData = JSON.parse(text);

                        const data = fetchedData.map((dataset) => ({
                            x: dataset.time, 
                            y: dataset.angles_of_attack, 
                            type: 'scatter', 
                            mode: 'lines'
                        }));

                        setData(data);
                    }
                })
                .catch(error => console.error('Error fetching data:', error));
        }, fetchInterval);

        return () => clearInterval(intervalId);
    }, []);

    if (!Plot) {
        return <div>Loading...</div>;
    }

    return (
        <div className="plot-container">
            <Plot data={data} layout={layout} />
        </div>
    );
};

export default AverageAnglesOfAttack;