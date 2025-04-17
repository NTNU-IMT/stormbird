use std::fs::File;
use std::io::{Write, BufWriter, Error};

use super::Wake;

use std::collections::HashMap;

impl Wake {
    pub fn write_wake_data_to_file_if_activated(&self, time_step_index: usize) {
        if self.settings.write_wake_data_to_file {
            let file_path = format!("{}/wake_{}.vtp", self.settings.wake_files_folder_path, time_step_index);
            let write_result = self.write_wake_to_vtk_file(&file_path);

            match write_result {
                Ok(_) => {},
                Err(e) => {
                    log::error!("Error writing wake data to file: {}", e);
                }
            }
        }
    }

    /// Export the wake geometry as an obj file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_obj_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        write!(writer, "o wake\n")?;

        for i in 0..self.points.len(){
            write!(
                writer, 
                "v {} {} {}\n", 
                self.points[i][0], 
                self.points[i][1], 
                self.points[i][2]
            )?;
        };

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            let indices = self.panel_point_indices(stream_index, span_index);

            write!(
                writer, 
                "f {} {} {} {}\n", 
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
        write!(writer, "<?xml version=\"1.0\"?>\n")?;
        write!(writer, "<VTKFile type=\"PolyData\" version=\"0.1\" byte_order=\"LittleEndian\">\n")?;
        write!(writer, "\t<PolyData>\n")?;
        write!(
            writer, 
            "\t\t<Piece NumberOfPoints=\"{}\" NumberOfVerts=\"0\" NumberOfLines=\"0\" NumberOfStrips=\"0\" NumberOfPolys=\"{}\">\n", 
            nr_points, 
            nr_faces
        )?;

        // Write points
        write!(writer, "\t\t\t<Points>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Float32\" NumberOfComponents=\"3\" format=\"ascii\">\n")?;
        for i in 0..nr_points {
            write!(
                writer, 
                "\t\t\t\t\t{} {} {}\n", 
                self.points[i][0], 
                self.points[i][1], 
                self.points[i][2]
            )?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</Points>\n")?;

        // Write faces
        write!(writer, "\t\t\t<Polys>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">\n")?;

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.indices.reverse_panel_index(panel_index);

            let indices = self.panel_point_indices(stream_index, span_index);

            write!(
                writer, 
                "\t\t\t\t\t{} {} {} {}\n", 
                indices[0], 
                indices[1], 
                indices[2], 
                indices[3]
            )?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"offsets\" format=\"ascii\">\n")?;
        for i in 0..nr_faces {
            write!(writer, "\t\t\t\t\t{}\n", (i+1)*4)?;
        }
        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</Polys>\n")?;

        // Write strength
        write!(writer, "\t\t\t<CellData Scalars=\"strength\">\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Float32\" Name=\"strength\" format=\"ascii\">\n")?;
        for i in 0..nr_faces {
            write!(writer, "\t\t\t\t\t{}\n", self.strengths[i])?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</CellData>\n")?;

        write!(writer, "\t\t</Piece>\n")?;
        write!(writer, "\t</PolyData>\n")?;
        write!(writer, "</VTKFile>\n")?;

        writer.flush()?;

        Ok(())
    }

    /// Exports the wake structure to a hashmap with the necessary fields for plotting the mesh 
    /// using the Plotly library. 
    pub fn export_to_plotly_mesh(&self) -> HashMap<String, Vec<f64>> {
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
            i.push(indices[0] as f64);
            j.push(indices[1] as f64);
            k.push(indices[2] as f64);

            i.push(indices[0] as f64);
            j.push(indices[2] as f64);
            k.push(indices[3] as f64);

            strength.push(self.strengths[panel_index]);
            strength.push(self.strengths[panel_index]); 
        }

        let mut out_data: HashMap<String, Vec<f64>> = HashMap::new();
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