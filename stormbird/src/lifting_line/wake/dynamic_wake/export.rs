// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)


use std::fs::File;
use std::io::{Write, BufWriter, Error};

use super::DynamicWake;
use crate::lifting_line::wake::vtk::polygons_as_vtk;

use std::collections::HashMap;

use stormath::type_aliases::Float;

impl DynamicWake {
    /// Export the wake geometry as an obj file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_obj_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        writeln!(writer, "o wake")?;

        for i in 0..self.points.len(){
            writeln!(
                writer, 
                "v {} {} {}", 
                self.points[i][0], 
                self.points[i][1], 
                self.points[i][2]
            )?;
        };

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            let indices = self.panel_point_indices(stream_index, span_index);

            writeln!(
                writer, 
                "f {} {} {} {}", 
                indices[0] + 1, 
                indices[1] + 1, 
                indices[2] + 1, 
                indices[3] + 1
            )?;
        }

        writer.flush()?;

        Ok(())
    }

    /// Export the wake geometry and strength as a VTK file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_vtk_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        let nr_points = self.points.len();
        let nr_faces  = self.strengths.len();

        // Header
        writeln!(writer, "<?xml version=\"1.0\"?>")?;
        writeln!(writer, "<VTKFile type=\"PolyData\" version=\"0.1\" byte_order=\"LittleEndian\">")?;
        writeln!(writer, "\t<PolyData>")?;
        writeln!(
            writer, 
            "\t\t<Piece NumberOfPoints=\"{}\" NumberOfVerts=\"0\" NumberOfLines=\"0\" NumberOfStrips=\"0\" NumberOfPolys=\"{}\">", 
            nr_points, 
            nr_faces
        )?;

        // Write points
        writeln!(writer, "\t\t\t<Points>")?;
        writeln!(writer, "\t\t\t\t<DataArray type=\"Float32\" NumberOfComponents=\"3\" format=\"ascii\">")?;
        for i in 0..nr_points {
            writeln!(
                writer, 
                "\t\t\t\t\t{} {} {}", 
                self.points[i][0], 
                self.points[i][1], 
                self.points[i][2]
            )?;
        }

        writeln!(writer, "\t\t\t\t</DataArray>")?;
        writeln!(writer, "\t\t\t</Points>")?;

        // Write faces
        writeln!(writer, "\t\t\t<Polys>")?;
        writeln!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">")?;

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            let indices = self.panel_point_indices(stream_index, span_index);

            writeln!(
                writer, 
                "\t\t\t\t\t{} {} {} {}", 
                indices[0], 
                indices[1], 
                indices[2], 
                indices[3]
            )?;
        }

        writeln!(writer, "\t\t\t\t</DataArray>")?;
        writeln!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"offsets\" format=\"ascii\">")?;
        for i in 0..nr_faces {
            writeln!(writer, "\t\t\t\t\t{}", (i+1)*4)?;
        }
        writeln!(writer, "\t\t\t\t</DataArray>")?;
        writeln!(writer, "\t\t\t</Polys>")?;

        // Write strength
        writeln!(writer, "\t\t\t<CellData Scalars=\"strength\">")?;
        writeln!(writer, "\t\t\t\t<DataArray type=\"Float32\" Name=\"strength\" format=\"ascii\">")?;
        for i in 0..nr_faces {
            writeln!(writer, "\t\t\t\t\t{}", self.strengths[i])?;
        }

        writeln!(writer, "\t\t\t\t</DataArray>")?;

        // Write viscous core length

        writeln!(writer, "\t\t\t\t<DataArray type=\"Float32\" Name=\"viscous_core_length\" format=\"ascii\">")?;
        for i in 0..nr_faces {
            writeln!(writer, "\t\t\t\t\t{}", self.panels_viscous_core_length[i])?;
        }

        writeln!(writer, "\t\t\t\t</DataArray>")?;
        writeln!(writer, "\t\t\t</CellData>")?;

        writeln!(writer, "\t\t</Piece>")?;
        writeln!(writer, "\t</PolyData>")?;
        writeln!(writer, "</VTKFile>")?;

        writer.flush()?;

        Ok(())
    }

    /// Serialize the wake geometry and strengths as a legacy VTK PolyData buffer, with one quad
    /// per panel, and the strength and viscous core length of each panel as cell data.
    ///
    /// Unlike [`DynamicWake::write_wake_to_vtk_file`], which writes an XML `.vtp` file to disk,
    /// this returns the data in memory as raw bytes. See [`polygons_as_vtk`] for more on the
    /// format.
    pub fn wake_as_vtk(&self, binary: bool) -> Vec<u8> {
        let polygons: Vec<Vec<usize>> = (0..self.strengths.len()).map(|panel_index| {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            self.panel_point_indices(stream_index, span_index).to_vec()
        }).collect();

        polygons_as_vtk(
            "Stormbird dynamic wake",
            &self.points,
            &polygons,
            &[
                ("strength", self.strengths.clone()),
                ("viscous_core_length", self.panels_viscous_core_length.clone()),
            ],
            binary
        )
    }

    /// Exports the wake structure to a hashmap with the necessary fields for plotting the mesh 
    /// using the Plotly library. 
    pub fn export_to_plotly_mesh(&self) -> HashMap<String, Vec<Float>> {
        let mut x = Vec::new();
        let mut y = Vec::new();
        let mut z = Vec::new();
        let mut i = Vec::new();
        let mut j = Vec::new();
        let mut k = Vec::new();
        let mut strength = Vec::new();

        for i in 0..self.points.len(){
            x.push(self.points[i][0]);
            y.push(self.points[i][1]);
            z.push(self.points[i][2]);
        }

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            let indices = self.panel_point_indices(stream_index, span_index);

            // Push two triangles for each panel
            i.push(indices[0] as Float);
            j.push(indices[1] as Float);
            k.push(indices[2] as Float);

            i.push(indices[0] as Float);
            j.push(indices[2] as Float);
            k.push(indices[3] as Float);

            strength.push(self.strengths[panel_index]);
            strength.push(self.strengths[panel_index]); 
        }

        let mut out_data: HashMap<String, Vec<Float>> = HashMap::new();
        out_data.insert("x".to_string(), x);
        out_data.insert("y".to_string(), y);
        out_data.insert("z".to_string(), z);
        out_data.insert("i".to_string(), i);
        out_data.insert("j".to_string(), j);
        out_data.insert("k".to_string(), k);
        out_data.insert("strength".to_string(), strength);

        out_data
    }
}