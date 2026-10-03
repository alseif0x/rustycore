//! Compile immutable source objects, not mutable placeholder checkout files.
//! Generated dependency copies belong only to Cargo OUT_DIR. No downloads.
use std::{fs, path::Path, process::Command};

const SOURCE: &str = "02245dcd245e7433e524577656177723d3e4992e";
const FILES: &[(&str, &str)] = &[
    ("dep/SFMT/SFMT.c", "SFMT.c"),
    ("dep/SFMT/SFMT.h", "SFMT.h"),
    ("dep/SFMT/SFMT-common.h", "SFMT-common.h"),
    ("dep/SFMT/SFMT-params.h", "SFMT-params.h"),
    ("dep/SFMT/SFMT-params19937.h", "SFMT-params19937.h"),
    ("dep/SFMT/SFMT-sse2.h", "SFMT-sse2.h"),
    ("dep/SFMT/LICENSE.txt", "SFMT-LICENSE.txt"),
    ("src/common/Define.h", "Define.h"),
    ("src/common/CompilerDefs.h", "CompilerDefs.h"),
    ("src/common/Utilities/Duration.h", "Duration.h"),
    ("src/common/Utilities/Random.h", "Random.h"),
    ("src/common/Utilities/SFMTRand.h", "SFMTRand.h"),
    ("src/common/Utilities/SFMTRand.cpp", "SFMTRand.cpp"),
];

fn source(root: &Path, path: &str) -> Vec<u8> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{SOURCE}:{path}")])
        .output()
        .expect("read pinned source object");
    assert!(
        output.status.success(),
        "Prepare the pinned target Git objects before enabling forever-spell-random; builds do not download/repair dependencies"
    );
    output.stdout
}

fn section<'a>(source: &'a str, start: &str, next: &str) -> &'a str {
    let position = source.find(start).expect("pinned source section start");
    let remainder = &source[position..];
    &remainder[..remainder.find(next).expect("pinned source section end")]
}

pub(super) fn build(manifest_dir: &Path) {
    for (variable, expected) in [
        ("CARGO_CFG_TARGET_OS", "linux"),
        ("CARGO_CFG_TARGET_ARCH", "x86_64"),
        ("CARGO_CFG_TARGET_ENV", "gnu"),
    ] {
        assert_eq!(
            std::env::var(variable).as_deref(),
            Ok(expected),
            "Forever source RNG currently requires the pinned Linux x64 GNU contract"
        );
    }
    println!("cargo:rerun-if-env-changed=FOREVER_CPP_SOURCE_ROOT");
    let root = std::env::var_os("FOREVER_CPP_SOURCE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("../../target/forever-cpp-reference"));
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"))
        .join("forever-source-rng");
    fs::create_dir_all(&output).expect("prepare generated source dependencies");
    for &(input, name) in FILES {
        fs::write(output.join(name), source(&root, input)).expect("copy immutable source object");
    }
    let random = String::from_utf8(source(&root, "src/common/Utilities/Random.cpp"))
        .expect("pinned public source encoding");
    // The actual source GetRng/frand/urand/urandweighted/rand32 bodies are retained verbatim,
    // with pristine headers/seeding/SFMT. No handwritten numerical clone.
    // ASSERT only checks max>=min here; the FFI boundary rejects it beforehand.
    let license = &random[..random.find("#include").expect("source license boundary")];
    let fragment = format!(
        "{license}#include \"Random.h\"\n#include \"SFMTRand.h\"\n#include <memory>\n#include <random>\n#include <cassert>\n#define ASSERT(condition) assert(condition)\n{}\n{}\n{}\n{}\n{}\n",
        section(
            &random,
            "namespace\n{\nconstexpr RandomEngine engine;",
            "\n\nint32 irand"
        ),
        section(
            &random,
            "float frand(float min, float max)\n{",
            "\n\nMilliseconds randtime"
        ),
        section(&random, "uint32 rand32()\n{", "\n\nfloat rand_norm"),
        section(
            &random,
            "uint32 urand(uint32 min, uint32 max)\n{",
            "\n\nuint32 urandms"
        ),
        section(
            &random,
            "uint32 urandweighted(size_t count, double const* chances)\n{",
            "\n\nnamespace"
        ),
    );
    fs::write(output.join("SourceRandom.cpp"), fragment).expect("write source RNG fragment");
    let directory = manifest_dir.join("src/forever/spell_random");
    println!("cargo:rerun-if-changed={}", directory.display());
    cc::Build::new()
        .cargo_metadata(false)
        .define("SFMT_MEXP", "19937")
        .define("HAVE_SSE2", None)
        .flag("-msse2")
        .include(&output)
        .file(output.join("SFMT.c"))
        .compile("rustycore_forever_sfmt");
    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .define("SFMT_MEXP", "19937")
        .define("HAVE_SSE2", None)
        .flag("-msse2")
        .flag("-ffp-contract=off")
        .include(&output)
        .file(output.join("SFMTRand.cpp"))
        .file(output.join("SourceRandom.cpp"))
        .file(directory.join("bridge.cpp"))
        .compile("rustycore_forever_spell_random");
    // Static dependency follows its consumer in the GNU linker order.
    println!("cargo:rustc-link-lib=static=rustycore_forever_sfmt");
    // Source headers/library above retain their notices. The SFMT license must
    // accompany redistributed binaries, not just be left inside Cargo cache.
}
