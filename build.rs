fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/icon.ico")
            .set("ProductName", "whipnext")
            .set("InternalName", "whipnext.exe")
            .set("OriginalFilename", "whipnext.exe");
        resource.compile().expect("compile Windows resources");
    }
}
