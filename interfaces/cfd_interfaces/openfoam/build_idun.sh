#!/bin/bash

#SBATCH --job-name=build
#SBATCH --partition=CPUQ
#SBATCH --account=share-sintef-ocean-t
#SBATCH --nodes=1
#SBATCH --ntasks-per-node=1
#SBATCH --mem=0G
#SBATCH --time=0:30:0
#SBATCH --error=error.log

WORKDIR=${SLURM_SUBMIT_DIR}
cd ${WORKDIR}

module purge
module load OpenFOAM/v2412-foss-2023a
source $FOAM_BASH

wmake src