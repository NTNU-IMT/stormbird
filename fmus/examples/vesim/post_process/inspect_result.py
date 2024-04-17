import pandas as pd

import numpy as numpy

import os

import argparse

from pathlib import Path

def list_field_names(df, key_in_name=''):
    for (colname, _) in df.items():
        if key_in_name in colname:
            print(colname)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Plot results.')
    parser.add_argument('--result-key', type=str, default='SOBC1', help='Key to filter result files.')
    parser.add_argument('--field-name-key', type=str, default='', help='Key to filter field names.')
    parser.add_argument('--with-stormbird', action='store_true', help='Run with stormbird.')
    args = parser.parse_args()

    if args.with_stormbird:
        output_path = Path('../output/output_with_stormbird')
    else:
        output_path = Path('../output/output_no_stormbird')
    
    all_output_files = os.listdir(output_path)

    files_of_interest = []
    for f in all_output_files:
        if args.result_key in f and '.csv' in f:
            files_of_interest.append(f)

    files_of_interest.sort()

    df = pd.read_csv(output_path / Path(files_of_interest[0]))

    list_field_names(df, args.field_name_key)