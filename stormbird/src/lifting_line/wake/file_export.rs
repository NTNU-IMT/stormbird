use std::fs::File;
use std::io::{Write, BufWriter, Error};

use super::Wake;

impl Wake {
    /// Export the wake geometry as an obj file
    ///
    /// # Argument
    /// * `file_path` - The path to the file to be written
    pub fn write_wake_to_obj_file(&self, file_path: &str) -> Result<(), Error> {
        let f = File::create(file_path)?;

        let mut writer = BufWriter::new(f);

        write!(writer, "o wake\n")?;

        for i in 0..self.wake_points.len(){
            write!(
                writer, 
                "v {} {} {}\n", 
                self.wake_points[i][0], 
                self.wake_points[i][1], 
                self.wake_points[i][2]
            )?;
        };

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(panel_index);

            let indices = self.panel_wake_point_indices(stream_index, span_index);

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

        let nr_points = self.wake_points.len();
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
                self.wake_points[i][0], 
                self.wake_points[i][1], 
                self.wake_points[i][2]
            )?;
        }

        write!(writer, "\t\t\t\t</DataArray>\n")?;
        write!(writer, "\t\t\t</Points>\n")?;

        // Write faces
        write!(writer, "\t\t\t<Polys>\n")?;
        write!(writer, "\t\t\t\t<DataArray type=\"Int32\" Name=\"connectivity\" format=\"ascii\">\n")?;

        for panel_index in 0..self.strengths.len() {
            let (stream_index, span_index) = self.reverse_panel_index(panel_index);

            let indices = self.panel_wake_point_indices(stream_index, span_index);

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
}