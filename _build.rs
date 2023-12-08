use core::panic;
use std::io::Error;
use std::process::Command;

fn main() -> Result<(), Error> {
    let queries_path = "queries";
    println!("cargo:rerun-if-changed={queries_path}");

    let mut command = Command::new("scripts/cornucopia.sh");
    let output = command.spawn()?.wait_with_output()?;
    if output.status.code() != Some(0) {
        panic!("{:?}", output);
    }
    Ok(())
}
