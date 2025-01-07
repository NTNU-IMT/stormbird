'use client';

import React from 'react';

interface ClearDataProps {
    serverAddress: string;
}

const ClearData: React.FC<ClearDataProps> = ({ serverAddress }) => {
    const handleButtonClick = () => {
        fetch(`http://${serverAddress}/clear-data`, { method: 'POST' })
            .catch(error => console.error('Error calling API:', error));
    };

    return (
        <div>
            <button onClick={handleButtonClick}>Clear Data</button>
        </div>
    );
}

export default ClearData;