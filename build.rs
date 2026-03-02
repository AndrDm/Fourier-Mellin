/**************************************************************************/
/* Rust Build File                                                        */
/*                                                                        */
/**************************************************************************/

use bindgen::MacroTypeVariation;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::{env, fs, path::Path};

fn main() -> std::io::Result<()> {
	// PROFILE ==============================================================
	// Get the output directory (e.g., target/debug or target/release)
	let out_dir = env::var("OUT_DIR").unwrap();
	// OUT_DIR is something like .../target/debug/build/yourcrate-xxxxxx/out
	// To get to target/debug or target/release:
	let _profile = env::var("PROFILE").unwrap();
	let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
	let target_dir = Path::new(&out_dir)
		.ancestors()
		.nth(3) // Go up 3 levels: out -> build -> debug/release -> target
		.expect("Failed to determine target directory");

	let src_img = Path::new(&manifest_dir).join("img");
	let dst_img = target_dir.join("img");

	// Remove old dst if exists (catches deletions)
	let _ = fs::remove_dir_all(&dst_img);
	fs::create_dir_all(&dst_img).expect("Failed to create dst img dir");

	// Recursive copy
	copy_dir_all(&src_img, &dst_img).expect("Failed to copy img folder");

	// UIR FILE ==============================================================
	println!("cargo:rerun-if-changed=cvi/forier_mellin.uir");
	let dest_dir = target_dir.join("bin");
	fs::create_dir_all(&dest_dir).expect("Failed to create destination directory");

	let src_file = Path::new(&manifest_dir).join("cvi/fourier_mellin.uir");
	let dest_file = dest_dir.join("fourier_mellin.uir");
	fs::copy(&src_file, &dest_file).expect("Could not copy uir file");
	println!("src {:?}, dst {:?}", src_file, dest_file);

	// HEADER FILE ===========================================================
	// BindGen
	println!("cargo:rerun-if-changed=cvi/fourier_mellin.h");

	let input_path = "cvi/fourier_mellin.h";
	let output_path = "cvi/fourier_mellin_bind.h";

	// Handle the Result properly here
	let input_file = File::open(input_path)?;
	let reader = BufReader::new(input_file);

	let mut output_file = File::create(output_path)?;

	for line in reader.lines() {
		let line = line?;
		if line.trim_start().starts_with("#include")
			|| line.trim_start().starts_with("int  CVICALLBACK")
			|| line.trim_start().starts_with("void CVICALLBACK")
		{
			continue;
		}
		writeln!(output_file, "{}", line)?;
	}

	println!("Filtered content written to {}", output_path);

	let bindings = bindgen::Builder::default()
		.header("cvi/fourier_mellin_bind.h")
		.default_macro_constant_type(MacroTypeVariation::Signed) // to i32
		.generate()
		.expect("Unable to generate bindings");

	let bindings_out_path = "src/fourier_mellin.rs";
	bindings.write_to_file(bindings_out_path).expect("Couldn't write bindings!");

	// Read the file, prepend the attribute, and write it back
	let mut contents = std::fs::read_to_string(bindings_out_path)?;
	contents = format!("#![allow(dead_code)]\n{}", contents);
	std::fs::write(bindings_out_path, contents)?;

	println!("Updated file with #![allow(dead_code)] at the top.");

	// official ext support
	println!("cargo:rustc-link-lib=lib\\cvirt");
	println!("cargo:rustc-link-lib=lib\\cvisupp");
	println!("cargo:rustc-link-lib=lib\\cviauto");
	println!("cargo:rustc-link-lib=lib\\cvi");
	println!("cargo:rustc-link-lib=lib\\cvistart");

	println!("cargo:rustc-link-arg=lib\\fourier_mellin.obj");

	println!("cargo:rustc-link-arg=lib\\ImageControl.obj");
	println!("cargo:rustc-link-lib=lib\\instrsup_start");
	println!("cargo:rustc-link-arg=lib\\toolbox.obj");
	println!("cargo:rustc-link-arg=lib\\asynctmr.obj");

	println!("cargo:rustc-link-lib=lib\\nivision");

	println!("cargo:rustc-link-lib=user32");
	println!("cargo:rustc-link-lib=advapi32");
	println!("cargo:rustc-link-lib=gdi32");

	Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
	for entry in fs::read_dir(src)? {
		let entry = entry?;
		let ty = entry.file_type()?;
		let src_path = entry.path();
		let dst_path = dst.as_ref().join(entry.file_name());

		if ty.is_dir() {
			fs::create_dir_all(&dst_path)?;
			copy_dir_all(&src_path, &dst_path)?;
		} else {
			fs::copy(&src_path, &dst_path)?;
		}
	}
	Ok(())
}
