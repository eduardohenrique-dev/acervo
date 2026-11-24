fn main() {
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=app.manifest");
    let mut attrs = tauri_build::Attributes::new();
    // Só o release pede administrador; em debug o app roda sem e usa a varredura alternativa.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        attrs = attrs.windows_attributes(
            tauri_build::WindowsAttributes::new().app_manifest(include_str!("app.manifest")),
        );
    }
    tauri_build::try_build(attrs).expect("falha no tauri-build");
}
