// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Shared functionality to export wake geometry and data in the legacy VTK format.

use std::io::Write;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

/// Serialize a set of polygons, with scalar data for each polygon, as a legacy VTK PolyData
/// buffer.
///
/// The data is returned in memory as raw bytes, so it can be consumed directly (e.g. fed into a VTK
/// reader in pyvista) without touching the disk.
///
/// The `binary` switch selects between the ASCII and BINARY variants of the VTK legacy format. In
/// ASCII mode the returned bytes are valid UTF-8 text, while in binary mode they contain the
/// big-endian binary data blocks mandated by the format. `Vec<u8>` covers both cases uniformly.
///
/// # Arguments
/// * `title` - The title written in the header of the data
/// * `points` - The points of the polygons
/// * `polygons` - The indices of the points making up each polygon. The polygons can have
/// different numbers of points.
/// * `cell_scalars` - Named scalar values, with one value for each polygon
/// * `binary` - Whether to use the binary or ASCII variant of the format
pub fn polygons_as_vtk(
    title: &str,
    points: &[SpatialVector],
    polygons: &[Vec<usize>],
    cell_scalars: &[(&str, Vec<Float>)],
    binary: bool
) -> Vec<u8> {
    let mut w: Vec<u8> = Vec::new();

    let nr_points = points.len();
    let nr_faces  = polygons.len();

    // ------------------------------------------------------------------ //
    // Header (always ASCII in the VTK legacy format)                       //
    // ------------------------------------------------------------------ //
    writeln!(w, "# vtk DataFile Version 3.0").unwrap();
    writeln!(w, "{}", title).unwrap();
    writeln!(w, "{}", if binary { "BINARY" } else { "ASCII" }).unwrap();
    writeln!(w, "DATASET POLYDATA").unwrap();

    // ------------------------------------------------------------------ //
    // Points                                                               //
    // ------------------------------------------------------------------ //
    writeln!(w, "POINTS {} float", nr_points).unwrap();
    for point in points {
        let x = point[0] as f32;
        let y = point[1] as f32;
        let z = point[2] as f32;

        if binary {
            w.write_all(&x.to_be_bytes()).unwrap();
            w.write_all(&y.to_be_bytes()).unwrap();
            w.write_all(&z.to_be_bytes()).unwrap();
        } else {
            writeln!(w, "{} {} {}", x, y, z).unwrap();
        }
    }
    if binary { writeln!(w).unwrap(); }

    // ------------------------------------------------------------------ //
    // Polygons                                                             //
    //                                                                      //
    // The legacy POLYGONS section is `POLYGONS n_cells connectivity_size`,  //
    // where each cell is stored as `n_points i0 i1 ...`. The connectivity  //
    // size is therefore the sum of (n_points + 1) over all cells.          //
    // ------------------------------------------------------------------ //
    let connectivity_size: usize = polygons.iter().map(|polygon| polygon.len() + 1).sum();

    writeln!(w, "POLYGONS {} {}", nr_faces, connectivity_size).unwrap();
    for polygon in polygons {
        if binary {
            w.write_all(&(polygon.len() as i32).to_be_bytes()).unwrap();

            for index in polygon {
                w.write_all(&(*index as i32).to_be_bytes()).unwrap();
            }
        } else {
            let indices: Vec<String> = polygon.iter().map(|index| index.to_string()).collect();

            writeln!(w, "{} {}", polygon.len(), indices.join(" ")).unwrap();
        }
    }
    if binary { writeln!(w).unwrap(); }

    // ------------------------------------------------------------------ //
    // Cell data                                                            //
    // ------------------------------------------------------------------ //
    writeln!(w, "CELL_DATA {}", nr_faces).unwrap();

    for (name, values) in cell_scalars {
        writeln!(w, "SCALARS {} float 1", name).unwrap();
        writeln!(w, "LOOKUP_TABLE default").unwrap();
        for value in values {
            let v = *value as f32;
            if binary {
                w.write_all(&v.to_be_bytes()).unwrap();
            } else {
                writeln!(w, "{}", v).unwrap();
            }
        }
        if binary { writeln!(w).unwrap(); }
    }

    w
}
