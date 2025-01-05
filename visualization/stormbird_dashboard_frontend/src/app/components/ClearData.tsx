'use client';

import React, { useEffect, useState } from 'react';

const ClearData = () => {
    const handleButtonClick = () => {
        fetch('http://localhost:8080/clear-data', { method: 'POST' })
            .catch(error => console.error('Error calling API:', error));
    };

    return (
        <div>
            <button onClick={handleButtonClick}>Clear Data</button>
        </div>
    );

}

export default ClearData;