/*
cizrna: Generate an AsciiDoc release notes document from tracking tickets.
Copyright (C) 2023  Marek Suchánek  <msuchane@redhat.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

/*!
a Cizrna subcommand that initializes an empty directory with all the necessary Cizrna configuration files.

This makes it more convenient to set up a new release notes project from scratch.
*/

use std::fs;
use std::path::Path;

use color_eyre::{eyre::WrapErr, Result};
use include_dir::{include_dir, Dir, DirEntry};

/// The `example` directory in the Cizrna source repository.
static EXAMPLE_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/example");

/// Copy example configuration files into the selected directory.
///
/// If the directory doesn't exist, create it.
pub fn initialize_directory(dir: &Path) -> Result<()> {
    if !dir.exists() {
        log::info!("The directory does not exist. Creating.");
        fs::create_dir_all(dir).wrap_err("Failed to create the project directory.")?;
    }

    // The absolute path to the selected target directory.
    let absolute_target = dir.canonicalize()?;
    log::info!(
        "Initializing a release notes project in {}",
        absolute_target.display()
    );

    let files = display_files(&EXAMPLE_DIR, &absolute_target);
    log::info!("Creating files:\n{}", files);

    EXAMPLE_DIR
        .extract(dir)
        .wrap_err("Failed to copy files to the project directory.")?;

    Ok(())
}

/// List all file paths from the example directory as a newline-separated string.
fn display_files(dir: &Dir, abs_target: &Path) -> String {
    let rel_paths = files_in_entries(dir.entries());

    let abs_paths = rel_paths.iter().map(|rel_path| abs_target.join(rel_path));

    let strings: Vec<String> = abs_paths
        .map(|path| format!("• {}", path.display()))
        .collect();
    strings.join("\n")
}

/// Return all the file paths from the example directory, recursively.
fn files_in_entries<'a>(entries: &'a [DirEntry<'a>]) -> Vec<&'a Path> {
    // This has to be written iteratively.
    // I tried the functional iter-map-collect chain and ran into:
    // error[E0277]: a value of type `Vec<&std::path::Path>` cannot be built
    // from an iterator over elements of type `Vec<&std::path::Path>`
    let mut results = Vec::new();

    for entry in entries {
        let mut files = files_in_entry(entry);
        results.append(&mut files);
    }
    results
}

/// A helper function for `files_in_entries` that distinguishes files and recursive directories.
fn files_in_entry<'a>(entry: &'a DirEntry) -> Vec<&'a Path> {
    match entry {
        DirEntry::File(file) => {
            vec![file.path()]
        }
        DirEntry::Dir(dir) => files_in_entries(dir.entries()),
    }
}
