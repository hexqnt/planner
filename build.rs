#[derive(serde::Deserialize)]
struct Lock {
    package: Vec<Package>,
}

#[derive(serde::Deserialize)]
struct Package {
    name: String,
    version: String,
}

fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let text = std::fs::read_to_string("Cargo.lock").expect("Unable to read Cargo.lock");
    let lock: Lock = toml::from_str(&text).expect("Unable to parse Cargo.lock");
    let package = lock
        .package
        .iter()
        .find(|package| package.name == "holidays-ru")
        .expect("holidays-ru is missing from Cargo.lock");
    println!("cargo:rustc-env=HOLIDAYS_RU_VERSION={}", package.version);
}
