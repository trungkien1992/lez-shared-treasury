fn main() {
    println!("cargo:rerun-if-env-changed=LIFECYCLE_PREBUILT_GUEST");
    if let Some(path) = std::env::var_os("LIFECYCLE_PREBUILT_GUEST") {
        let path = std::fs::canonicalize(path).expect("prebuilt guest is missing");
        println!("cargo:rerun-if-changed={}", path.display());
        let digest = std::process::Command::new("sha256sum").arg(&path).output()
            .expect("sha256sum is needed to verify the prebuilt guest");
        assert!(digest.status.success(), "guest hash verification command failed");
        let hash = String::from_utf8(digest.stdout).expect("non-UTF8 hash output");
        assert_eq!(hash.split_whitespace().next().unwrap(),
            "fff9878b2215978e6be4bd62ac1f89a4c3b1b351bf4f0d61b308eaa0c30e321c",
            "prebuilt guest differs from the archived, tested lifecycle program");
        let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(output.join("methods.rs"), format!(
            "pub const LIFECYCLE_GUEST_ELF: &[u8] = include_bytes!({:?});\n",
            path.to_str().expect("non-UTF8 guest path"))).unwrap();
        return;
    }
    risc0_build::embed_methods();
}
