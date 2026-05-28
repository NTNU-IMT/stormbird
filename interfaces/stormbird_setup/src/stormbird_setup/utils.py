import numpy as np

def revolutions_per_second_from_spin_ratio(
    *,
    spin_ratio: float,
    diameter: float,
    velocity: float
):
    '''
    Helper function to convert spin ratio to revolutions per second
    '''
    circumference = np.pi * diameter
    tangential_velocity = velocity * spin_ratio
            
    revolutions_per_second = -tangential_velocity / circumference 

    return revolutions_per_second
