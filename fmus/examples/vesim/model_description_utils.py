import xml.etree.ElementTree as ET

def add_initial_value(initial_values, *, variable: str, value: float | bool | str | int):
    initial_value_elem = ET.SubElement(initial_values, 'InitialValue')
    initial_value_elem.set('variable', variable)
    initial_value_elem.tail = '\n\t\t\t'
    initial_value_elem.text = '\n\t\t\t\t\t'

    if isinstance(value, float):
        value_elem = ET.SubElement(initial_value_elem, 'Real')
        value_elem.set('value', str(value))
    elif isinstance(value, str):
        value_elem = ET.SubElement(initial_value_elem, 'String')
        value_elem.set('value', value)
    elif isinstance(value, bool):
        value_elem = ET.SubElement(initial_value_elem, 'Boolean')

        if value:
            value_elem.set('value', '1')
        else:
            value_elem.set('value', '0')
    else:
        raise ValueError('Unsupported type:', type(value))
    
    value_elem.tail = '\n\t\t\t\t'

def add_simulator(
    simulators, 
    *, 
    name: str, 
    source: str,
    initial_variables: dict[str, float | bool | str | int] = {},
):
    simulator = ET.SubElement(simulators, 'Simulator')
    simulator.set('name', name)
    simulator.set('source', source)
    simulator.tail = '\n\t\t'
    simulator.text = '\n\t\t\t'

    initial_values = ET.SubElement(simulator, 'InitialValues')
    initial_values.tail = '\n\t\t'

    if len(initial_variables) > 0:
        initial_values.text = '\n\t\t\t\t'
    else:
        initial_values.text = '\n\t\t\t'

    for var_name, var_value in initial_variables.items():
        add_initial_value(initial_values, variable=var_name, value=var_value)
        

    return simulator

def add_connection(connections, *, simulators: list[str], variables: list[str]):
    '''
    Adds a connection between specified variables in two simulators.
    '''

    if len(simulators) > 2 or len(variables) > 2:
        raise ValueError('Only connections between two simulators are supported.')
    
    connection = ET.SubElement(connections, 'VariableConnection')
    connection.tail = '\n\t'
    connection.text = '\n\t\t\t'

    for index, (sim, var) in enumerate(zip(simulators, variables)):
        source_variable = ET.SubElement(connection, 'Variable')
        source_variable.set('simulator', sim)
        source_variable.set('name', var)

        if index == 0:
            source_variable.tail = '\n\t\t\t'
        else:
            source_variable.tail = '\n\t\t'