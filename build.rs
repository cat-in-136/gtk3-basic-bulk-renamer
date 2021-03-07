use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

fn build_glib_compile_resource<P: AsRef<Path>>(source: P) -> Result<(), Box<dyn Error>> {
    let source = source.as_ref();
    let parent_dir = source.parent().unwrap();

    println!("cargo:rerun-if-changed={}", source.to_str().unwrap());
    let dependencies = Command::new("glib-compile-resources")
        .args(&["--generate-dependencies", source.to_str().unwrap()])
        .stderr(Stdio::piped())
        .output()?
        .stdout;
    for dependency in String::from_utf8(dependencies)?.lines() {
        let dep_file = parent_dir.join(dependency);
        println!("cargo:rerun-if-changed={}", dep_file.to_str().unwrap());
    }

    Command::new("glib-compile-resources")
        .args(&[source.strip_prefix(parent_dir)?.to_str().unwrap()])
        .current_dir(parent_dir)
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;

    if source.with_extension("").exists() {
        Ok(())
    } else {
        panic!(
            "Fail to generate {}",
            source.with_extension("").to_str().unwrap()
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={}", file!());

    build_glib_compile_resource("src/win/resource.gresource.xml")?;

    Ok(())
}
