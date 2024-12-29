document.getElementById('clearButton').addEventListener('click', async () => {
    try {
        const response = await fetch('/clear-data', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                'clear': true
            })
        });

        if (response.ok) {
            console.log('Success: Data cleared');
            location.reload();
        } else {
            console.error('Error:', response.statusText);
        }
    } catch (error) {
        console.error('Error:', error);
    }
});