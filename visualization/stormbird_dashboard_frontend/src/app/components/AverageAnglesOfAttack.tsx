'use client';

import React, { useEffect, useState } from 'react';

interface ClearDataProps {
    serverAddress: string;
}

const AverageAnglesOfAttack: React.FC<ClearDataProps> = ({ serverAddress }) => {
    const [data, setData] = useState([]);
    
    const [layout, setLayout] = useState(
        {
            title: 'Angle of attack measurements on each wing as a function of time', 
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
            fetch(`http://${serverAddress}/get-angle-of-attack-measurements`)
                .then(response => response.text())
                .then(text => {
                    if (text) {
                        const fetchedData = JSON.parse(text);

                        console.log(fetchedData);

                        const data = fetchedData.map((dataset) => ({
                            x: dataset.time, 
                            y: dataset.angle_of_attack_measurement, 
                            type: 'scatter', 
                            mode: 'lines'
                        }));

                        setData(data);
                    }
                })
                .catch(error => console.error('Error fetching data:', error));
        }, fetchInterval);

        return () => clearInterval(intervalId);
    }, [serverAddress]);

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