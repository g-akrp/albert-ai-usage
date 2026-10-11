use std::{env, fs, path::PathBuf};

mod build_icon;
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_icon.rs");
    println!("cargo:rerun-if-changed=../../Resources/AppIcon.icns");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let components = version
        .split('.')
        .map(|s| s.parse::<u16>().unwrap())
        .collect::<Vec<_>>();
    let numbers = format!("{},{},{},0", components[0], components[1], components[2]);
    let assembly = format!("{version}.0");
    let manifest = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><assemblyIdentity type="win32" name="g.akrp.AIUsage" version="{assembly}"/><trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security></trustInfo><compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1"><application><supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/></application></compatibility><application xmlns="urn:schemas-microsoft-com:asm.v3"><windowsSettings><dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/PM</dpiAware><dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2,PerMonitor</dpiAwareness></windowsSettings></application></assembly>"#
    );
    fs::write(out.join("app.manifest"), manifest).unwrap();
    fs::write(out.join("app.ico"), build_icon::icon()).unwrap();
    let manifest = out
        .join("app.manifest")
        .to_string_lossy()
        .replace('\\', "/");
    let icon = out.join("app.ico").to_string_lossy().replace('\\', "/");
    let rc = format!(
        r#"#include <winres.h>
1 RT_MANIFEST "{manifest}"
1 ICON "{icon}"
VS_VERSION_INFO VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "CompanyName", "g.akrp\0"
   VALUE "FileDescription", "AI Usage for Windows\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "ProductName", "AI Usage\0"
   VALUE "ProductVersion", "{version}\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#
    );
    let resource = out.join("app.rc");
    fs::write(&resource, rc).unwrap();
    embed_resource::compile(resource, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
