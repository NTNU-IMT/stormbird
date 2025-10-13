#!/bin/bash

cp -r 0.org 0

blockMesh
snappyHexMesh -overwrite

decomposePar

mpirun -np $number_of_threads pimpleFoam -parallel

reconstructPar

rm -fr proc*

postProcess -func Q