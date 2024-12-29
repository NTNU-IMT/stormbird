'use client';

import React, { useEffect } from 'react';
import dynamic from 'next/dynamic';

// Dynamically import Plotly without SSR
const Plotly = dynamic(() => import('plotly.js-dist-min'), { ssr: false });

const ForcePlot = () => {
    useEffect(() => {
        const loadPlotly = async () => {
            const Plotly = await import('plotly.js-dist-min');

            const x_trace = {
                x: [],
                y: [],
                type: 'scatter',
                mode: 'lines',
                name: 'Force in x direction',
            };
            
            const y_trace = {
                x: [],
                y: [],
                type: 'scatter',
                mode: 'lines',
                name: 'Force in y direction',
            };
            
            const layout = {
                title: 'Integrated forces as a function of time',
                xaxis: { title: 'Time' },
                yaxis: { title: 'Value' }
            };
            
            const fetchInterval = 500;
            
            const divName = 'forcePlot';
            
            Plotly.newPlot(divName, [x_trace, y_trace], layout);
            
            const intervalId = setInterval(() => {
                fetch('http://localhost:8080/get-forces')
                    .then(response => response.text())
                    .then(text => {
                        if (text) {
                            const data = JSON.parse(text);
                
                            x_trace.x = data.time;
                            x_trace.y = data.force_x;
                
                            y_trace.x = data.time;
                            y_trace.y = data.force_y;
                            
                            Plotly.redraw(divName);
                        }
                    })
                    .catch(error => console.error('Error fetching data:', error));
            }, fetchInterval);

            return () => clearInterval(intervalId);
        };

        loadPlotly();
    }, []);

    return <div id="forcePlot" />;
};

export default ForcePlot;