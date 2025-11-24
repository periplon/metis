fn main() {
    // Tell Cargo to rerun this build script (and thus recompile) when the UI dist changes
    println!("cargo:rerun-if-changed=ui/dist");
}
