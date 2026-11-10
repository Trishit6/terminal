//! Build script for the KERNEXT desktop application.
//!
//! On `*-pc-windows-gnu` targets some MinGW-w64 distributions (WinLibs,
//! current MSYS2) link a `default-manifest.o` into every executable through
//! the GCC endfile specs. That default manifest has no Common Controls v6
//! side-by-side dependency, and the linker keeps it over Tauri's application
//! manifest. The binary then dies at startup with STATUS_ENTRYPOINT_NOT_FOUND
//! because `TaskDialogIndirect` is missing from the v5 `comctl32.dll`.
//!
//! The manifest object is selected by `%:if-exists(default-manifest.o%s)`,
//! which searches the startfile prefixes with `-B` directories first, so this
//! script compiles a replacement object carrying the required dependency and
//! prepends its directory to the search path.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Application manifest used for `*-pc-windows-gnu` builds.
///
/// This keeps the Common Controls dependency Tauri's default manifest
/// provides plus the `longPathAware` and `asInvoker` settings the MinGW
/// default manifest carried, so nothing regresses when it is replaced.
const GNU_APP_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <asmv3:application xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
    <asmv3:windowsSettings xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">
      <longPathAware>true</longPathAware>
    </asmv3:windowsSettings>
  </asmv3:application>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
"#;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let is_windows_gnu = target_os == "windows" && target_env == "gnu";

    // The GNU build embeds its manifest through `shadow_default_manifest`
    // below, so Tauri's own app manifest is disabled there to keep exactly
    // one manifest in the binary. Every other target keeps the Tauri
    // default (Common Controls v6) untouched.
    let attributes = if is_windows_gnu {
        tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest())
    } else {
        tauri_build::Attributes::new()
    };

    if let Err(error) = tauri_build::try_build(attributes) {
        panic!("failed to run tauri-build: {error}");
    }

    if is_windows_gnu {
        if let Err(error) = shadow_default_manifest() {
            // A missing override only restores the previous behavior, so
            // this is a warning rather than a hard failure.
            println!("cargo:warning={error}");
        }
    }
}

/// Compiles a `default-manifest.o` carrying the Common Controls v6 dependency
/// into the build output directory and prepends that directory to the GCC
/// startfile search path.
///
/// The endfile spec `%{!shared:%:if-exists(default-manifest.o%s)}` resolves
/// `-B` directories first, so the linker picks this object instead of the
/// toolchain's and the manifest survives linking.
fn shadow_default_manifest() -> Result<(), String> {
    let out_dir =
        PathBuf::from(env::var("OUT_DIR").map_err(|error| format!("OUT_DIR is not set: {error}"))?);

    let manifest = out_dir.join("default-manifest.xml");
    let resource = out_dir.join("default-manifest.rc");

    fs::write(&manifest, GNU_APP_MANIFEST)
        .map_err(|error| format!("failed to write {}: {error}", manifest.display()))?;
    fs::write(&resource, "1 24 \"default-manifest.xml\"\n")
        .map_err(|error| format!("failed to write {}: {error}", resource.display()))?;

    compile_manifest_object(&out_dir)?;

    // A trailing separator is required: GCC treats each `-B` value as a
    // literal prefix concatenated with every searched file name.
    let prefix = format!("-B{}{}", out_dir.display(), std::path::MAIN_SEPARATOR);
    println!("cargo:rustc-link-arg={prefix}");

    Ok(())
}

/// Runs `windres` inside `out_dir` to produce `default-manifest.o`.
fn compile_manifest_object(out_dir: &Path) -> Result<(), String> {
    let target = env::var("TARGET").map_err(|error| format!("TARGET is not set: {error}"))?;
    let windres_target = if target.starts_with("i586") || target.starts_with("i686") {
        "pe-i386"
    } else if target.starts_with("aarch64") || target.starts_with("arm64ec") {
        "pe-aarch64-little"
    } else {
        "pe-x86-64"
    };

    let candidates = [
        "windres".to_string(),
        "x86_64-w64-mingw32-windres".to_string(),
        "i686-w64-mingw32-windres".to_string(),
        "aarch64-w64-mingw32-windres".to_string(),
    ];

    let mut last_error = String::new();

    for candidate in &candidates {
        let output = match Command::new(candidate)
            .arg("-i")
            .arg("default-manifest.rc")
            .arg("-O")
            .arg("coff")
            .arg("-o")
            .arg("default-manifest.o")
            .arg("--target")
            .arg(windres_target)
            .current_dir(out_dir)
            .output()
        {
            Ok(output) => output,
            Err(error) => {
                last_error = format!("could not run {candidate}: {error}");
                continue;
            }
        };

        if output.status.success() {
            return Ok(());
        }

        last_error = format!(
            "{candidate} failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    Err(format!(
        "no working windres found to build default-manifest.o: {last_error}"
    ))
}
