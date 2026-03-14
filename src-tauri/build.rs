fn main() {
    tauri_build::build();

    // Skip platform-specific runtime setup for tests
    let is_test_build =
        std::env::var("CARGO_CFG_TEST").is_ok() || std::env::var("CARGO_PRIMARY_PACKAGE").is_err();

    if is_test_build {
        return;
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=native=libs");

        // Add rpaths for both the app's libs directory and system library paths
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../../libs");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/System/Library/Frameworks");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Developer/CommandLineTools/usr/lib");

        // Copy libs directory to target directory for development
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let target_dir = std::path::Path::new(&out_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("debug");

        // Create libs directory in target
        let libs_dir = target_dir.join("libs");
        if let Err(e) = std::fs::create_dir_all(&libs_dir) {
            println!("cargo:warning=Failed to create libs directory: {}", e);
            return;
        }

        // Copy ONNX runtime library from bin/onnxruntime directory
        let src_lib = std::path::Path::new("bin/onnxruntime").join("libonnxruntime.1.22.0.dylib");
        let dst_lib = libs_dir.join("libonnxruntime.1.22.0.dylib");

        if src_lib.exists() {
            if let Err(e) = std::fs::copy(&src_lib, &dst_lib) {
                println!("cargo:warning=Failed to copy ONNX runtime library: {}", e);
                return;
            }
        } else {
            println!(
                "cargo:warning=ONNX runtime library not found at: {}",
                src_lib.display()
            );
        }

        // Also copy to release directory for release builds
        let release_dir = target_dir.parent().unwrap().join("release");
        let release_libs_dir = release_dir.join("libs");

        if let Err(e) = std::fs::create_dir_all(&release_libs_dir) {
            println!(
                "cargo:warning=Failed to create release libs directory: {}",
                e
            );
            return;
        }

        let release_dst_lib = release_libs_dir.join("libonnxruntime.1.22.0.dylib");
        if src_lib.exists() {
            if let Err(e) = std::fs::copy(&src_lib, &release_dst_lib) {
                println!(
                    "cargo:warning=Failed to copy ONNX runtime library to release directory: {}",
                    e
                );
                return;
            }
        }

        // Create bin directory in target for FFmpeg binaries
        let bin_dir = target_dir.join("bin");
        if let Err(e) = std::fs::create_dir_all(&bin_dir) {
            println!("cargo:warning=Failed to create bin directory: {}", e);
            return;
        }

        // Copy FFmpeg binaries and preserve permissions
        let ffmpeg_files = ["ffmpeg", "ffprobe"];
        for file in ffmpeg_files.iter() {
            let src = std::path::Path::new("bin").join(file);
            let dst = bin_dir.join(file);

            if src.exists() {
                // Copy the file
                if let Err(e) = std::fs::copy(&src, &dst) {
                    println!("cargo:warning=Failed to copy {}: {}", file, e);
                    continue;
                }

                // Get the original permissions
                if let Ok(metadata) = std::fs::metadata(&src) {
                    let mut perms = metadata.permissions();
                    use std::os::unix::fs::PermissionsExt;
                    perms.set_mode(0o755); // rwxr-xr-x

                    // Set the same permissions on the copy
                    if let Err(e) = std::fs::set_permissions(&dst, perms) {
                        println!(
                            "cargo:warning=Failed to set permissions for {}: {}",
                            file, e
                        );
                    }
                }
            } else {
                println!("cargo:warning={} not found at: {}", file, src.display());
            }
        }

        // Also copy FFmpeg binaries to release directory
        let release_bin_dir = release_dir.join("bin");
        if let Err(e) = std::fs::create_dir_all(&release_bin_dir) {
            println!(
                "cargo:warning=Failed to create release bin directory: {}",
                e
            );
            return;
        }

        for file in ffmpeg_files.iter() {
            let src = std::path::Path::new("bin").join(file);
            let dst = release_bin_dir.join(file);

            if src.exists() {
                // Copy the file
                if let Err(e) = std::fs::copy(&src, &dst) {
                    println!("cargo:warning=Failed to copy {} to release: {}", file, e);
                    continue;
                }

                // Get the original permissions
                if let Ok(metadata) = std::fs::metadata(&src) {
                    let mut perms = metadata.permissions();
                    use std::os::unix::fs::PermissionsExt;
                    perms.set_mode(0o755); // rwxr-xr-x

                    // Set the same permissions on the copy
                    if let Err(e) = std::fs::set_permissions(&dst, perms) {
                        println!(
                            "cargo:warning=Failed to set permissions for {} in release: {}",
                            file, e
                        );
                    }
                }
            }
        }
    }
}
