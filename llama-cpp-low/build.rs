use std::env;
use std::path::PathBuf;

const SUBMODULE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/llama.cpp");

fn main() {
    let cuda = std::env::var("CARGO_FEATURE_CUDA").unwrap_or(String::new());

    let submodule_dir = &PathBuf::from(SUBMODULE_DIR);

    if !submodule_dir.join("CMakeLists.txt").exists() {
        eprintln!("did you run 'git submodule update --init' ?");
        std::process::exit(1);
    }

    let mut cmake = cmake::Config::new(&submodule_dir);

    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        if cuda == "1" {
            cmake.configure_arg("-DGGML_CUDA=ON");
        }
    }

    cmake.profile("Release").build_target("llama-server");
    let dst = cmake.build();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Need to test with METAL environment
    // std::fs::copy(
    //     submodule_dir.join("src/ggml/ggml-metal.metal"),
    //     out_path.join("../../../ggml-metal.metal"),
    // )
    // .expect("Couldn't copy ggml-metal.metal");
    // std::fs::copy(
    //     submodule_dir.join("ggml-common.h"),
    //     out_path.join("../../../ggml-common.h"),
    // )
    // .expect("Couldn't copy ggml-common.h");
    std::fs::copy(
        dst.join("build/bin/llama-server"),
        out_path.join("../../../server"),
    )
    .expect(&format!("Couldn't copy server from {:?}", dst));
}
