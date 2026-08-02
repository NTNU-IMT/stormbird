use super::Simulation;

use std::fs::File;
use std::io::{BufWriter, Write};

impl Simulation {
    pub fn export_fields_as_vtk(&self, file_path: &str, binary: bool) {
        let [nx, ny, nz] = self.grid.interior_shape;
        let [x0, y0, z0] = self.grid.start_point.0;
        let [dx, dy, dz] = self.grid.cell_length.0;

        let n_points = (nx + 1) * (ny + 1) * (nz + 1);
        let n_cells  = nx * ny * nz;

        let file = File::create(file_path).expect("export_fields_as_vtk: could not create file");
        let mut w = BufWriter::new(file);

        // ------------------------------------------------------------------ //
        // ASCII file header (always ASCII in the VTK legacy format)           //
        // ------------------------------------------------------------------ //
        writeln!(w, "# vtk DataFile Version 3.0").unwrap();
        writeln!(w, "Stormflow CFD Simulation").unwrap();
        writeln!(w, "{}", if binary { "BINARY" } else { "ASCII" }).unwrap();
        writeln!(w, "DATASET STRUCTURED_GRID").unwrap();
        writeln!(w, "DIMENSIONS {} {} {}", nx + 1, ny + 1, nz + 1).unwrap();
        writeln!(w, "POINTS {} double", n_points).unwrap();

        // ------------------------------------------------------------------ //
        // Cell-corner coordinates                                              //
        //                                                                      //
        // Corner (px, py, pz) sits at:                                        //
        //   (x0 + px*dx,  y0 + py*dy,  z0 + pz*dz)                           //
        //                                                                      //
        // VTK ordering: x varies fastest, z varies slowest.                   //
        // ------------------------------------------------------------------ //
        for pz in 0..=(nz) {
            for py in 0..=(ny) {
                for px in 0..=(nx) {
                    let x: f64 = x0 as f64 + px as f64 * dx as f64;
                    let y: f64 = y0 as f64 + py as f64 * dy as f64;
                    let z: f64 = z0 as f64 + pz as f64 * dz as f64;

                    if binary {
                        w.write_all(&x.to_be_bytes()).unwrap();
                        w.write_all(&y.to_be_bytes()).unwrap();
                        w.write_all(&z.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{} {} {}", x, y, z).unwrap();
                    }
                }
            }
        }

        // Newline separator required before the next ASCII keyword in binary mode.
        if binary { writeln!(w).unwrap(); }

        // ------------------------------------------------------------------ //
        // Cell data                                                            //
        // ------------------------------------------------------------------ //
        writeln!(w, "CELL_DATA {}", n_cells).unwrap();

        // --- Pressure (scalar, stored at cell centers on the extended grid) ---
        writeln!(w, "SCALARS pressure double 1").unwrap();
        writeln!(w, "LOOKUP_TABLE default").unwrap();

        let pressure = self.pressure_solver.pressure_ref();

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let flat = self.grid.flat_index_on_extended_grid_from_interior_indices([ix, iy, iz]);

                    let p: f64 = pressure[flat] as f64;

                    if binary {
                        w.write_all(&p.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{}", p).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        // --- Velocity (interpolated from staggered faces to cell centers) ---
        //
        // The MAC staggered layout stores each component at the face in its
        // own direction.  Concretely, velocity_x[ex, ey, ez] sits on the
        // *right* x-face of cell (ex, ey, ez) — i.e. between cells (ex, ey, ez)
        // and (ex+1, ey, ez).  This is confirmed by the pressure-projection
        // update:
        //
        //   velocity_x[current] -= (dt/rho) * (p[x_pos] - p[current]) / dx
        //
        // Cell-centre interpolation therefore averages the two bracketing faces:
        //
        //   u_c = 0.5 * (velocity_x[ex-1, ey, ez] + velocity_x[ex, ey, ez])
        //   v_c = 0.5 * (velocity_y[ex, ey-1, ez] + velocity_y[ex, ey, ez])
        //   w_c = 0.5 * (velocity_z[ex, ey, ez-1] + velocity_z[ex, ey, ez])
        //
        writeln!(w, "VECTORS velocity double").unwrap();

        let velocity = &self.velocity_solver.velocity;

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let [ex, ey, ez] = self.grid.extended_indices_from_interior_indices([ix, iy, iz]);

                    let flat_c      = self.grid.flat_index_on_extended_grid([ex,     ey,     ez    ]);
                    let flat_x_left = self.grid.flat_index_on_extended_grid([ex - 1, ey,     ez    ]);
                    let flat_y_bot  = self.grid.flat_index_on_extended_grid([ex,     ey - 1, ez    ]);
                    let flat_z_back = self.grid.flat_index_on_extended_grid([ex,     ey,     ez - 1]);

                    let u: f64 = (0.5 * (velocity[flat_x_left][0] + velocity[flat_c][0])) as f64;
                    let v: f64 = (0.5 * (velocity[flat_y_bot][1]  + velocity[flat_c][1])) as f64;
                    let ww: f64 = (0.5 * (velocity[flat_z_back][2] + velocity[flat_c][2])) as f64;

                    if binary {
                        w.write_all(&u.to_be_bytes()).unwrap();
                        w.write_all(&v.to_be_bytes()).unwrap();
                        w.write_all(&ww.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{} {} {}", u, v, ww).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        writeln!(w, "VECTORS body_force double").unwrap();

        let body_force = &self.velocity_solver.body_force;

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let i_flat = self.grid.flat_index_on_extended_grid_from_interior_indices(
                        [ix, iy, iz]
                    );
                    
                    let fx = body_force[i_flat][0] as f64;
                    let fy = body_force[i_flat][1] as f64;
                    let fz = body_force[i_flat][2] as f64;

                    if binary {
                        w.write_all(&fx.to_be_bytes()).unwrap();
                        w.write_all(&fy.to_be_bytes()).unwrap();
                        w.write_all(&fz.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{} {} {}", fx, fy, fz).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        // --- Pressure (scalar, stored at cell centers on the extended grid) ---
        writeln!(w, "SCALARS sdf double 1").unwrap();
        writeln!(w, "LOOKUP_TABLE default").unwrap();

        let signed_distance_function = &self.velocity_solver.signed_distance_function;

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let flat = self.grid.flat_index_on_extended_grid_from_interior_indices([ix, iy, iz]);
                    let sdf: f64 = signed_distance_function[flat] as f64;

                    if binary {
                        w.write_all(&sdf.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{}", sdf).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        // --- Signed distance function for slip surfaces (scalar) ---
        writeln!(w, "SCALARS sdf_slip double 1").unwrap();
        writeln!(w, "LOOKUP_TABLE default").unwrap();

        let signed_distance_function_slip = &self.velocity_solver.signed_distance_function_slip;

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let flat = self.grid.flat_index_on_extended_grid_from_interior_indices([ix, iy, iz]);
                    let sdf_slip: f64 = signed_distance_function_slip[flat] as f64;

                    if binary {
                        w.write_all(&sdf_slip.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{}", sdf_slip).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        // --- Normals for slip surfaces (vector) ---
        writeln!(w, "VECTORS normals_slip double").unwrap();

        let normals_slip_surfaces = &self.velocity_solver.normals_slip_surfaces;

        for iz in 0..nz {
            for iy in 0..ny {
                for ix in 0..nx {
                    let flat = self.grid.flat_index_on_extended_grid_from_interior_indices([ix, iy, iz]);

                    let nx_val: f64 = normals_slip_surfaces[flat][0] as f64;
                    let ny_val: f64 = normals_slip_surfaces[flat][1] as f64;
                    let nz_val: f64 = normals_slip_surfaces[flat][2] as f64;

                    if binary {
                        w.write_all(&nx_val.to_be_bytes()).unwrap();
                        w.write_all(&ny_val.to_be_bytes()).unwrap();
                        w.write_all(&nz_val.to_be_bytes()).unwrap();
                    } else {
                        writeln!(w, "{} {} {}", nx_val, ny_val, nz_val).unwrap();
                    }
                }
            }
        }

        if binary { writeln!(w).unwrap(); }

        w.flush().expect("export_fields_as_vtk: failed to flush output");
    }
}
