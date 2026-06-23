fn main() {
    // AVFoundation is needed at runtime to look up `AVCaptureDevice` for the
    // microphone-permission request (see request_microphone_access in lib.rs).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
    }
    tauri_build::build()
}
