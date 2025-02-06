'use client';

import React, { useEffect, useState } from 'react';

interface ClearDataProps {
    serverAddress: string;
}

const CirculationDistributionPlot: React.FC<ClearDataProps> = ({ serverAddress }) => {
    const [data, setData] = useState([]);
    
    const [layout, setLayout] = useState(
        {
            title: 'Circulation distribution', 
            xaxis: { title: 'Z coordinate' }, 
            yaxis: { title: 'Value' },
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
            fetch(`http://${serverAddress}/get-circulation-distribution`)
                .then(response => response.text())
                .then(text => {
                    if (text) {
                        const fetchedData = JSON.parse(text);

                        const newData = fetchedData.map((dataset) => ({
                            x: dataset.ctrl_points_z, 
                            y: dataset.circulation_strength, 
                            type: 'scatter', 
                            mode: 'lines'
                        }));

                        setData(newData);
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

export default CirculationDistributionPlot;