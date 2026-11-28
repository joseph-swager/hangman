// Embeds the app icon in hangman.exe. Does nothing on other platforms.
fn main() {
    println!("cargo:rerun-if-changed=../../packaging/windows/hangman.ico");
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("../../packaging/windows/hangman.ico")
        .compile()
        .expect("failed to embed the Windows icon");
}
