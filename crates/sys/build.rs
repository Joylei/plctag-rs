// plctag-rs
//
// a rust wrapper of libplctag, with rust style APIs and useful extensions.
// Copyright: 2020-2021, Joylei <leingliu@gmail.com>
// License: MIT

extern crate bindgen;
extern crate cmake;
extern crate pkg_config;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let is_static = check_static();
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
        let source: PathBuf = "libplctag".into();
        let mut config = cmake::Config::new(&source);
        // do not build examples
        config.define("BUILD_EXAMPLES", "0");
        if is_static {
            config.static_crt(true);
        }
        let out_dir = config.build();
        eprintln!("cmake build out dir: {:?}", &out_dir);
        let header_file = source.join("src").join("lib").join("libplctag.h");
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
        .whitelist_var("PLCTAG_.*")
        .whitelist_function("plc_tag_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .rustfmt_bindings(true)
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

/// check if static build in the order of:
/// PLCTAG_STATIC, PLCTAG_DYNAMIC, rustflags: +crt-static
fn check_static() -> bool {
    if let Some(v) = get_env_bool("LIBPLCTAG_STATIC") {
        return v;
    }
    if let Some(v) = get_env_bool("LIBPLCTAG_DYNAMIC") {
        return !v;
    }
    cfg!(target_feature = "crt-static")
}

fn get_env_bool(key: &str) -> Option<bool> {
    env::var(key).ok().map(|v| {
        let v = v.to_lowercase();
        if v == "1" || v == "true" || v == "on" {
            true
        } else {
            false
        }
    })
}
