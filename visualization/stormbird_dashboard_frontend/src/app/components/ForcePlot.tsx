'use client';

import React, { useEffect, useState } from 'react';

interface ClearDataProps {
    serverAddress: string;
}

const ForcePlot: React.FC<ClearDataProps> = ({ serverAddress }) => {
    const [data, setData] = useState(
        [
            { x: [], y: [], type: 'scatter', mode: 'lines', name: 'Force in x direction' }, 
            { x: [], y: [], type: 'scatter', mode: 'lines', name: 'Force in y direction' }
        ]
    );
    
    const [layout, setLayout] = useState(
        {
            title: 'Integrated forces as a function of time', 
            xaxis: { title: 'Time step' }, 
            yaxis: { title: 'Value [N]' },
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
            fetch(`http://${serverAddress}/get-forces`)
                .then(response => response.text())
                .then(text => {
                    if (text) {
                        const fetchedData = JSON.parse(text);

                        setData([
                            { x: fetchedData.time, y: fetchedData.force_x, type: 'scatter', mode: 'lines', name: 'Force in x direction' },
                            { x: fetchedData.time, y: fetchedData.force_y, type: 'scatter', mode: 'lines', name: 'Force in y direction' }
                        ]);
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

export default ForcePlot;