fn main() {
    // whisper.cpp requires std::filesystem (macOS 10.15+)
    // CMAKE_OSX_DEPLOYMENT_TARGET must be set for cmake-based C++ deps
    std::env::set_var("MACOSX_DEPLOYMENT_TARGET", "11.0");
    std::env::set_var("CMAKE_OSX_DEPLOYMENT_TARGET", "11.0");
    tauri_build::build()
}
