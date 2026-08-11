// use std::{env, fs, io, path::Path};

// fn cp_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
//     fs::create_dir_all(&dst)?;

//     for entry in fs::read_dir(&src)? {
//         let entry = entry?;
//         let src_path = entry.path();
//         let dst_path = dst.as_ref().join(entry.file_name());

//         if entry.file_type()?.is_file() {
//             fs::copy(src_path, dst_path)?;
//         } else {
//             cp_dir(src_path, dst_path)?
//         }
//     }

//     Ok(())
// }

// fn main() {
//     let source_dir = Path::new(r"bin\");

//     let profile = env::var("PROFILE").unwrap(); // debug 或 release
//     let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
//     let target_dir = &Path::new(&manifest_dir)
//         .join("target")
//         .join(&profile)
//         .join(source_dir);

//     println!("cargo:warning=🔥");

//     cp_dir(source_dir, target_dir).unwrap();
// }

fn main() {}
