fn main() {
    /*
    let library = HashMap::from([(
        "lucide".to_string(),
        PathBuf::from(lucide_slint::lib()),
    )]);

     */
    let config = slint_build::CompilerConfiguration::new()
        // .with_library_paths(library)
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);

    slint_build::compile_with_config(
        "ui/main.slint",
            config,
    ).expect("Slint build failed (build.rs)");
}
