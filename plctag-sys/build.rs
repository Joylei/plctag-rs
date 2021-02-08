extern crate bindgen;

use std::env;
use std::fs;
use std::path;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    let (lib_path, lib_file) =
        prob_lib(&env::var("LIBPLCTAG_PATH").expect("env LIBPLCTAG_PATH not found"))
            .expect("failed to prob library path");
    println!("cargo:rustc-link-lib=plctag");
    println!("cargo:rustc-link-search={}", lib_path.to_string_lossy());
    println!("cargo:rerun-if-changed=libplctag.h");
    let bindings = bindgen::Builder::default()
        .header("libplctag.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    eprintln!("OUT_DIR={:?}", out_path);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let target_dir = find_target_profile_dir(out_path)
        .map(|x| x.to_string_lossy().to_string())
        .unwrap();
    eprintln!("target profile dir={}", target_dir);
    //copy lib to target dir
    let dest_file = PathBuf::from(target_dir).join(lib_file.file_name().unwrap());

    #[cfg(target_os = "windows")]
    fs::copy(lib_file, dest_file).unwrap();
}

fn prob_lib(lib_path: &str) -> Option<(PathBuf, PathBuf)> {
    let dir = PathBuf::from(lib_path);
    let dir1 = dir.clone();
    #[cfg(target_os = "windows")]
    {
        let file = dir.join("plctag.lib");
        if file.is_file() && file.exists() {
            Some((dir, dir1.join("plctag.dll")))
        } else {
            let dir = dir.join("Release");
            let dir1 = dir.clone();
            let file = dir.join("plctag.lib");
            if file.is_file() && file.exists() {
                Some((dir, dir1.join("plctag.dll")))
            } else {
                None
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let dir1 = dir.clone();
        let file = dir.join("libplctag.a");
        if file.is_file() && file.exists() {
            Some((dir, dir1.join("libplctag.dylib")))
        } else {
            None
        }
    }
    #[cfg(target_os = "unix")]
    {
        let dir1 = dir.clone();
        let file = dir.join("libplctag.a");
        if file.is_file() && file.exists() {
            Some((dir, dir1.join("libplctag.so")))
        } else {
            None
        }
    }
}

fn find_target_profile_dir(dir: PathBuf) -> Option<PathBuf> {
    //out dir looks like ...\plctag-rs\target\debug\build\XXXXX
    //profile dir looks like ...\plctag-rs\target\debug\
    let mut dir = dir;
    loop {
        if let Some(p) = dir.parent() {
            let buf = p.to_path_buf();
            if buf.ends_with("build") {
                return Some(buf.parent().unwrap().to_path_buf());
            }
            dir = buf;
        } else {
            return None;
        }
    }
}
