import numpy as np

'''
Empirical models for the lift and drag on a rotor sail based on the equations presented in:

Tillig et al. (2020) "Design, operation and analysis of wind-assisted cargo ships", 
Ocean Engineering, 
link: https://www.sciencedirect.com/science/article/pii/S0029801820306077
'''


def tillig_lift_coefficient(spin_ratio: float) -> float:
    coefficients = [
        -0.1039,
        3.1309,
        -0.9817,
        0.1145,
        -0.0046
    ]

    cl = 0.0
    for i in range(len(coefficients)):
        cl += coefficients[i] * spin_ratio**(i+1)
    
    return cl

def tillig_drag_coefficient(spin_ratio: float) -> float:
    coefficients = [
        0.6375,
        -1.641,
        1.7243,
        -0.4424,
        0.0464,
        -0.0017
    ]
    
    cd = 0.0

    for i in range(len(coefficients)):
        cd += coefficients[i] * spin_ratio**i
    
    return cd