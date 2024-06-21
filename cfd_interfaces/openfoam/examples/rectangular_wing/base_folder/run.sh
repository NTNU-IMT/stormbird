#!/bin/bash

rm -fr 0
rm -fr 5
rm -fr 10
rm -fr 15
rm -fr 20
cp -r 0.org 0

blockMesh
snappyHexMesh -overwrite

decomposePar

mpirun -np 6 pimpleFoam -parallel

reconstructPar

rm -fr proc*

postProcess -func Q