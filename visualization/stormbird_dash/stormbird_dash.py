from dash import Dash, html, dcc
from dash.dependencies import Input, Output, State
import plotly.graph_objects as go
from flask import request, jsonify
import json
from collections import deque
import threading
import time

# Initialize Dash app
app = Dash(__name__)
server = app.server  # We'll need this to add Flask endpoints

# Use deque with maxlen for efficient data storage
data_buffer = {
    't': deque(maxlen=1000),
    'fx': deque(maxlen=1000),
    'fy': deque(maxlen=1000)
}

# Lock for thread-safe data updates
data_lock = threading.Lock()

# Layout
app.layout = html.Div([
    html.H1("Simulation Data"),
    dcc.Graph(id='live-graph'),
    dcc.Interval(id='graph-update', interval=100),  # 100ms refresh
    dcc.Store(id='data-store')  # Hidden div for storing data
])

# Add REST endpoint for receiving simulation data
@server.route('/update-data', methods=['POST'])
def update_data():
    if request.method == 'POST':
        new_data = request.json
        
        with data_lock:
            if len(data_buffer['t']) == 0:
                t = 0
            else:
                t = data_buffer['t'][-1] + 1

            data_buffer['t'].append(t)
            data_buffer['fx'].append(new_data['integrated_forces'][0]['total']['x'])
            data_buffer['fy'].append(new_data['integrated_forces'][0]['total']['y'])
            
        return jsonify({"status": "success"}), 200

# Callback to update graph
@app.callback(
    Output('live-graph', 'figure'),
    Input('graph-update', 'n_intervals')
)
def update_graph(n):
    with data_lock:
        x_data = list(data_buffer['t'])
        y_data = list(data_buffer['fy'])
    
    return {
        'data': [{
            'x': x_data,
            'y': y_data,
            'type': 'scatter',
            'mode': 'lines+markers'
        }],
        'layout': {
            'title': 'Real-time Simulation Data',
            'xaxis': {'title': 'Time'},
            'yaxis': {'title': 'Value'}
        }
    }

if __name__ == '__main__':
    app.run_server(debug=True, port=8050)

