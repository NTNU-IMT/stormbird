from pathlib import Path
import os

def find_output_file(folder_path: Path, simulator_name: str) -> Path:
    files_in_folder = os.listdir(folder_path)

    output_file_path = Path()
    for file in files_in_folder:
        if simulator_name in file and '.csv' in file:
            output_file_path = folder_path / file

    return output_file_path


def convert_lists_to_string(file_path: Path) -> Path:
    with open(file_path, 'r') as f:
        lines = f.readlines()

    new_lines = []
    for line in lines:
        if '[' in line:
            line = line.replace('[', '"[')
        if ']' in line:
            line = line.replace(']', ']"')

        new_lines.append(line)

    out_path = file_path.parent / (file_path.stem + '_stringified' + file_path.suffix)

    with open(out_path, 'w') as f:
        f.writelines(new_lines)

    return out_path
