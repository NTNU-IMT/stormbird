//! Checks that all the example setup files can be read, so that they are kept in sync with the
//! input format. The simulations are not run, but the grid, and the line force model and the
//! controller of the actuator line model, if present, are built, as they are cheap to build and
//! contain additional checks of the input.

use stormflow::simulation::builder::SimulationBuilder;

/// All the JSON files in the subfolders of the examples folder
fn example_files() -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();

    for folder in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/examples")).unwrap() {
        let folder = folder.unwrap().path();

        if !folder.is_dir() {
            continue;
        }

        for file in std::fs::read_dir(&folder).unwrap() {
            let file = file.unwrap().path();

            if file.extension().is_some_and(|extension| extension == "json") {
                files.push(file);
            }
        }
    }

    files
}

#[test]
fn all_example_setup_files_can_be_read() {
    let files = example_files();

    assert!(!files.is_empty());

    for file in files {
        let builder = SimulationBuilder::from_json_file(file.to_str().unwrap())
            .unwrap_or_else(|error| panic!("Failed to read {}: {:?}", file.display(), error));

        match &builder.actuator_line {
            Some(actuator_line) => {
                builder.grid.build_from_line_force_model_builder(&actuator_line.line_force_model);

                let line_force_model = actuator_line.line_force_model.build();

                if let Some(controller) = &actuator_line.controller {
                    controller.build(&line_force_model).unwrap_or_else(
                        |error| panic!("Invalid controller in {}: {:?}", file.display(), error)
                    );
                }
            },
            None => {
                builder.grid.build_from_internal_length().unwrap_or_else(
                    |error| panic!("Invalid grid in {}: {:?}", file.display(), error)
                );
            },
        }
    }
}
