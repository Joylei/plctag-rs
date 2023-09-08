// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2022, Joylei <leingliu@gmail.com>
// License: MIT

extern crate bindgen;
extern crate cmake;
extern crate pkg_config;

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[cfg(target_os = "windows")]
use std::{ffi::OsStr, path::Component};

fn main() {
    // check if static build in the order of:
    // PLCTAG_STATIC, PLCTAG_DYNAMIC, rustflags: +crt-static
    let is_static = get_env_bool("LIBPLCTAG_STATIC").unwrap_or(false)
        || get_env_bool("LIBPLCTAG_DYNAMIC").map_or(false, |v| !v)
        || cfg!(target_feature = "crt-static");

    if is_static {
        eprintln!("static build");
    }
    let (lib_path, header_file) = if let Ok(lib_path) = env::var("LIBPLCTAG_PATH") {
        eprintln!("use lib path from env:LIBPLCTAG_PATH");
        let lib_path: PathBuf = lib_path.into();
        let header_file = "libplctag.h".to_owned();
        println!("cargo:rerun-if-changed={}", header_file);
        (lib_path, header_file)
    } else {
        let source_dir = {
            let source_dir = "libplctag";
            // fix publish issue: Build scripts should not modify anything outside of OUT_DIR
            let dst_dir = PathBuf::from(env::var("OUT_DIR").unwrap()).join("libplctag");
            dir_copy(source_dir, &dst_dir).expect("failed to copy libplctag to OUT_DIR");
            dst_dir
        };
        let mut config = cmake::Config::new(&source_dir);
        // do not build examples
        config.define("BUILD_EXAMPLES", "0");
        if is_static {
            config.static_crt(true);
        }
        let out_dir = config.build();
        eprintln!("cmake build out dir: {:?}", &out_dir);
        let header_file = source_dir.join("src").join("lib").join("libplctag.h");
        println!("cargo:rerun-if-changed={}", header_file.display());
        (out_dir, header_file.display().to_string())
    };
    println!("cargo:rerun-if-env-changed=LIBPLCTAG_STATIC");
    println!("cargo:rerun-if-env-changed=LIBPLCTAG_DYNAMIC");
    if cfg!(target_os = "windows") && is_static {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=static=plctag_static");
    } else {
        println!("cargo:rustc-link-lib=plctag");
    }
    println!("cargo:rustc-link-search={}", lib_path.display());
    println!("cargo:rustc-link-search={}", lib_path.join("lib").display());
    println!(
        "cargo:rustc-link-search={}",
        lib_path.join("Release").display()
    );

    //generate bindings
    let bindings = bindgen::Builder::default()
        .header(header_file)
        .allowlist_var("PLCTAG_.*")
        .allowlist_function("plc_tag_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    eprintln!("OUT_DIR={:?}", out_path);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    #[cfg(target_os = "windows")]
    if !is_static {
        install_lib_files(lib_path, out_path);
    }
}

#[cfg(target_os = "windows")]
fn install_lib_files(lib_path: impl AsRef<Path>, out_path: impl AsRef<Path>) {
    let folders = &["", "lib", "Release"];
    let mut dll_file = None;
    for folder in folders {
        let path = lib_path.as_ref().join(folder).join("plctag.dll");
        if path.is_file() && path.exists() {
            dll_file = Some(path);
            break;
        }
    }
    if let Some(dll_file) = dll_file {
        let target_dir = find_target_profile_dir(out_path).unwrap();
        eprintln!("target profile dir={}", target_dir.display());
        //copy lib to target dir
        let dest_file = target_dir.join(dll_file.file_name().unwrap());
        fs::copy(dll_file, dest_file).unwrap();
    } else {
        eprintln!("plctag.dll not found");
    }
}

#[cfg(target_os = "windows")]
fn find_target_profile_dir<'a>(dir: impl AsRef<Path> + 'a) -> Option<PathBuf> {
    //out dir looks like ...\plctag-rs\target\debug\build\XXXXX
    //profile dir looks like ...\plctag-rs\target\debug\
    let target = Some(Component::Normal(OsStr::new("build")));
    let mut dir = dir.as_ref();
    loop {
        if let Some(p) = dir.parent() {
            let last_part = p.components().last();
            if last_part == target {
                return p.parent().map(|v| v.to_owned());
            }
            dir = p;
        } else {
            return None;
        }
    }
}

fn get_env_bool(key: &str) -> Option<bool> {
    env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_ref(), "1" | "true" | "on" | "yes"))
}

fn is_file_newer(a: &Path, b: &Path) -> bool {
    match (a.symlink_metadata(), b.symlink_metadata()) {
        (Ok(meta_a), Ok(meta_b)) => {
            meta_a.modified().unwrap_or_else(|_| SystemTime::now())
                > meta_b.modified().unwrap_or(SystemTime::UNIX_EPOCH)
        }
        _ => false,
    }
}

fn dir_copy(source_dir: impl AsRef<Path>, dst_dir: impl AsRef<Path>) -> io::Result<()> {
    let source_dir = source_dir.as_ref();
    let dst_dir = dst_dir.as_ref();
    if !source_dir.exists() {
        return Ok(());
    }
    if !dst_dir.exists() {
        fs::create_dir(dst_dir)?;
        fs::set_permissions(dst_dir, source_dir.metadata()?.permissions())?;
    }
    //eprintln!("cp src: {}", source_dir.display());
    //eprintln!("cp dst: {}", dst_dir.display());
    for entry in (source_dir.read_dir()?).flatten() {
        if let Ok(meta) = entry.metadata() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            let dst = dst_dir.join(name);
            //eprintln!("{}", dst.display());

            if meta.is_dir() {
                dir_copy(entry.path(), dst)?;
            } else if !dst.exists() || is_file_newer(entry.path().as_ref(), dst.as_ref()) {
                fs::copy(&entry.path(), &dst)?;
            }
        }
    }

    Ok(())
}
