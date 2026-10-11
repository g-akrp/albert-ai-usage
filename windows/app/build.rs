use std::{env, fs, path::PathBuf};

fn icon() -> Vec<u8> {
    let sizes = [16usize, 32, 48];
    let mut images = Vec::new();
    for size in sizes {
        let mut bitmap = Vec::new();
        bitmap.extend(40u32.to_le_bytes());
        bitmap.extend((size as i32).to_le_bytes());
        bitmap.extend((size as i32 * 2).to_le_bytes());
        bitmap.extend(1u16.to_le_bytes());
        bitmap.extend(32u16.to_le_bytes());
        bitmap.extend([0u8; 24]);
        let scale = (size / 16).max(1);
        let start = (size - 7 * scale) / 2;
        let top = (size - 11 * scale) / 2;
        for y in (0..size).rev() {
            for x in 0..size {
                let mut set = false;
                let mut brand = false;
                if x >= start && x < start + 7 * scale && y >= top && y < top + 11 * scale {
                    let gx = (x - start) / scale;
                    let gy = (y - top) / scale;
                    let row = if gy < 5 {
                        brand = true;
                        if gx < 3 {
                            [2, 5, 7, 5, 5][gy]
                        } else {
                            [7, 2, 2, 2, 7][gy]
                        }
                    } else if gy >= 6 {
                        [0, 0, 7, 0, 0][gy - 6]
                    } else {
                        0
                    };
                    let col = if gx < 3 { gx } else { gx.saturating_sub(4) };
                    set = gx != 3 && col < 3 && (row & (1 << (2 - col))) != 0;
                }
                bitmap.extend(if set {
                    if brand {
                        [244, 133, 66, 255]
                    } else {
                        [140, 140, 140, 255]
                    }
                } else {
                    [0; 4]
                });
            }
        }
        bitmap.extend(vec![0; size.div_ceil(32) * 4 * size]);
        images.push(bitmap);
    }
    let mut result = vec![0, 0, 1, 0, 3, 0];
    let mut offset = 6 + 16 * 3;
    for (size, image) in sizes.into_iter().zip(&images) {
        result.extend([size as u8, size as u8, 0, 0]);
        result.extend(1u16.to_le_bytes());
        result.extend(32u16.to_le_bytes());
        result.extend((image.len() as u32).to_le_bytes());
        result.extend((offset as u32).to_le_bytes());
        offset += image.len();
    }
    for image in images {
        result.extend(image);
    }
    result
}
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
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
    fs::write(out.join("app.ico"), icon()).unwrap();
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
