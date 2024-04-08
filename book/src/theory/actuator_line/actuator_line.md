# Actuator line
## Velocity sampling
To calculate the forces on each line element in the model, the local velocity needs to be estimated
from the CFD solver. The way the velocity is estimated can have an effect on the result, and 
different methods are in use the scientific litterature.

The classical approach simply interpolates the velocity at the control points directly. If the 
mesh resolution is suffcient and the body force projection method is symmetric in space, this should
give the right values.

However, there can be uncertanties when the mesh is coarse (remember: allowing for coarse mesh is 
the entire point of an actuator line method), and mistakes if the projection is non-symmetric.

Stormbird therefore uses a variant known as "integral sampling", taken from Churchfield et. al 
(2017)

## Force projection
To couple the line force model to a CFD solver, the forces estimated from the model needs to be
projected as volume forces in the momentum equation.

How this projection is done can have a large effect on the result, and this therefore requires some
care.


## References
- Churchfield, M., Schreck, S., Martines-Tossas, L. A., Meneveau, C., Spalart, P. R., 2017, An 
Advanced Actuator Line Method for Wind Energy Applications and Beyond, URL: 
<https://www.nrel.gov/docs/fy17osti/67611.pdf>