fn main() {
    let build_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    println!("cargo:rustc-env=BUILD_ID={build_id}");
}
