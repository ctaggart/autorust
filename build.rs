fn main() {
    println!("cargo:rustc-env=CFG_RELEASE=nightly");
    println!("cargo:rustc-env=CFG_RELEASE_CHANNEL=nightly");
}
