use std::{fs, io, path::Path};

/// A little setup func to copy the dist folder in frontend to a place where at compile time I can
/// access it, because I need to use absolute paths in the include_dir crate.
fn main() {
    recursive_copy("../gulfi_ui/templates/assets/", "dist/assets")
        .expect("Failed to copy files from frontend/dist to here");
}

fn recursive_copy(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    std::fs::create_dir_all(&dst)?;

    for entry in fs::read_dir(src).expect("Failed to read entries") {
        let entry = entry.expect("Failed to read the entry");
        let ty = entry.file_type()?;
        let dst_path = dst.as_ref().join(entry.file_name());

        if ty.is_dir() {
            eprintln!("{}", entry.path().display());
            recursive_copy(entry.path(), dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
