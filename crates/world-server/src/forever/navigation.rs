// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-31: explicit v56/v57 navigation resource admission.
//! Verified navigation resources for the isolated Forever runtime.

use anyhow::{Context, Result, bail, ensure};
use forever_casc::{Locale, integrity, package::Context as PackageContext};
use std::{ffi::OsStr, fs, path::Path, sync::Arc};
use wow_world::{
    WorldMMapPathfinderWorkerLikeCpp,
    pathfinder::{
        ForeverNavigation56Expected, ForeverNavigation57Expected, ForeverNavigationLimits,
    },
};

pub(super) struct NavigationRuntime {
    _worker: Arc<WorldMMapPathfinderWorkerLikeCpp>,
    pub selected_map: u32,
}

fn digest(value: &serde_json::Value) -> Result<[u8; 32]> {
    let text = value.as_str().context("navigation context digest")?;
    ensure!(text.len() == 64, "navigation context digest length");
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)
            .context("navigation context digest encoding")?;
    }
    Ok(output)
}

pub(super) fn load(
    package_root: &OsStr,
    plan_path: &OsStr,
    selected_map: &OsStr,
) -> Result<NavigationRuntime> {
    let package_root = Path::new(package_root);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(package_root.join("forever-package.json")).context("navigation manifest read")?,
    )
    .context("navigation manifest encoding")?;
    ensure!(
        manifest.get("status").and_then(|value| value.as_str()) == Some("complete"),
        "navigation package incomplete"
    );
    let value = manifest.get("context").context("navigation context")?;
    let locale = match value.get("locale").and_then(|value| value.as_str()) {
        Some("esES") => Locale::EsEs,
        Some("enUS") => Locale::EnUs,
        _ => bail!("navigation locale"),
    };
    let context = PackageContext {
        build: value
            .get("build")
            .and_then(|value| value.as_u64())
            .context("navigation build")?
            .try_into()
            .context("navigation build range")?,
        locale,
        source_sha256: digest(value.get("source_sha256").context("navigation source")?)?,
        plan_sha256: digest(
            value
                .get("plan_sha256")
                .context("navigation context plan")?,
        )?,
        producer_sha256: digest(
            value
                .get("producer_sha256")
                .context("navigation producer")?,
        )?,
        components: value
            .get("components")
            .and_then(|value| value.as_u64())
            .context("navigation components")?
            .try_into()
            .context("navigation components range")?,
    };
    ensure!(context.components == 32, "navigation component mask");
    let plan_bytes = fs::read(plan_path).context("navigation plan read")?;
    let plan_sha256 = integrity::digest(&plan_bytes);
    ensure!(
        context.plan_sha256 == plan_sha256,
        "navigation plan identity mismatch"
    );
    let selected_map = selected_map
        .to_str()
        .context("navigation map encoding")?
        .parse::<u32>()
        .context("navigation map identity")?;
    let worker = if plan_bytes.starts_with(b"forever-product\t56\n") {
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation56(
            package_root,
            ForeverNavigation56Expected {
                selected_map,
                locale,
                plan_sha256,
                context: Some(context),
            },
            ForeverNavigationLimits::default(),
        )
    } else if plan_bytes.starts_with(b"forever-product\t57\n") {
        WorldMMapPathfinderWorkerLikeCpp::spawn_forever_navigation57(
            package_root,
            ForeverNavigation57Expected {
                selected_map,
                locale,
                plan_sha256,
                context: Some(context),
            },
            ForeverNavigationLimits::default(),
        )
    } else {
        bail!("navigation product version")
    }
    .map_err(|_| anyhow::anyhow!("navigation startup rejected"))?;
    Ok(NavigationRuntime {
        _worker: Arc::new(worker),
        selected_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forever_casc::package::{Component, Limits, Run, Summary};
    use forever_navigation::{
        mmap,
        package_v56::{Coverage56, MapFact, Output, TileOutput},
        tilebuilder::{self, Coordinate},
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn navigation_resource_admission_binds_plan_and_package() {
        let root = std::env::temp_dir().join(format!(
            "forever-navigation-resource-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let package = root.join("package");
        let tile = Coordinate {
            row: 31,
            column: 31,
        };
        let params = tilebuilder::global_params(&[tile]).unwrap();
        let global = mmap::encode_global(48, params, &[], mmap::Limits::default()).unwrap();
        let source = Summary {
            context: PackageContext {
                build: 70450,
                locale: Locale::EnUs,
                source_sha256: [0x46; 32],
                plan_sha256: [0x51; 32],
                producer_sha256: [0x52; 32],
                components: 25,
            },
            files: 1,
            bytes: 128,
            inventory_sha256: [0x53; 32],
        };
        let context = PackageContext {
            build: 70450,
            locale: Locale::EnUs,
            source_sha256: integrity::digest(
                format!("rustycore-navigation-v56-sealed-v51\n{}\n", source.json()).as_bytes(),
            ),
            plan_sha256: integrity::digest(b"forever-product\t56\n"),
            producer_sha256: [0x35; 32],
            components: 32,
        };
        let coverage = Coverage56 {
            context,
            source,
            maps: vec![MapFact {
                id: 48,
                root: 48,
                state: "planned",
                tiles: vec![tile],
            }],
            outputs: vec![Output {
                map: 48,
                global: Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: global.len() as u64,
                    sha256: integrity::digest(&global),
                }),
                tiles: vec![TileOutput {
                    tile,
                    written: false,
                    bytes: 0,
                    sha256: [0; 32],
                }],
                partial_triangulations: 0,
                detail_center_fallbacks: 0,
                unset_height_samples: 0,
            }],
            builder: 0,
            save: 0,
        };
        let mut run = Run::create(&package, context, &[], Limits::default()).unwrap();
        run.put(
            Component::Navigation,
            forever_navigation::package_v56::COVERAGE_PATH,
            &coverage.render().unwrap(),
        )
        .unwrap();
        run.put(Component::Navigation, &mmap::global_path(48), &global)
            .unwrap();
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        let plan = root.join("plan.tsv");
        fs::write(&plan, b"forever-product\t56\n").unwrap();
        let runtime = load(package.as_os_str(), plan.as_os_str(), OsStr::new("48")).unwrap();
        assert_eq!(runtime.selected_map, 48);
        drop(runtime);

        fs::write(&plan, b"changed-plan\n").unwrap();
        assert!(load(package.as_os_str(), plan.as_os_str(), OsStr::new("48")).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn navigation_resource_admission_binds_v57_plan_and_package() {
        use forever_navigation::package_v57::{Coverage57, MapFact, Output};

        let root = std::env::temp_dir().join(format!(
            "forever-navigation57-resource-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let package = root.join("package");
        let tile = Coordinate {
            row: 31,
            column: 31,
        };
        let params = tilebuilder::global_params(&[tile]).unwrap();
        let global = mmap::encode_global(48, params, &[], mmap::Limits::default()).unwrap();
        let source = Summary {
            context: PackageContext {
                build: 70450,
                locale: Locale::EnUs,
                source_sha256: [0x46; 32],
                plan_sha256: [0x51; 32],
                producer_sha256: [0x52; 32],
                components: 25,
            },
            files: 1,
            bytes: 128,
            inventory_sha256: [0x53; 32],
        };
        let context = PackageContext {
            build: 70450,
            locale: Locale::EnUs,
            source_sha256: integrity::digest(
                format!("rustycore-navigation-v57-sealed-v51\n{}\n", source.json()).as_bytes(),
            ),
            plan_sha256: integrity::digest(b"forever-product\t57\n"),
            producer_sha256: [0x35; 32],
            components: 32,
        };
        let coverage = Coverage57 {
            context,
            source,
            maps: vec![MapFact {
                id: 48,
                root: 48,
                state: "planned",
                tiles: vec![tile],
            }],
            outputs: vec![Output {
                map: 48,
                global: Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: global.len() as u64,
                    sha256: integrity::digest(&global),
                }),
                tiles: vec![TileOutput {
                    tile,
                    written: false,
                    bytes: 0,
                    sha256: [0; 32],
                }],
                partial_triangulations: 0,
                detail_center_fallbacks: 0,
                unset_height_samples: 0,
            }],
            builder: 0,
            save: 0,
        };
        let mut run = Run::create(&package, context, &[], Limits::default()).unwrap();
        run.put(
            Component::Navigation,
            forever_navigation::package_v57::COVERAGE_PATH,
            &coverage.render().unwrap(),
        )
        .unwrap();
        run.put(Component::Navigation, &mmap::global_path(48), &global)
            .unwrap();
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        let plan = root.join("plan.tsv");
        fs::write(&plan, b"forever-product\t57\n").unwrap();
        let runtime = load(package.as_os_str(), plan.as_os_str(), OsStr::new("48")).unwrap();
        assert_eq!(runtime.selected_map, 48);
        drop(runtime);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "explicit real v56/v57 package required"]
    fn real_navigation_resource_admission() {
        let package = std::env::var_os("RCE_FOREVER_NAVIGATION57_PACKAGE")
            .or_else(|| std::env::var_os("RCE_FOREVER_NAVIGATION56_PACKAGE"))
            .expect("real package root required");
        let plan = std::env::var_os("RCE_FOREVER_NAVIGATION57_PLAN")
            .or_else(|| std::env::var_os("RCE_FOREVER_NAVIGATION56_PLAN"))
            .expect("real plan required");
        let map = std::env::var_os("RCE_FOREVER_NAVIGATION57_MAP")
            .or_else(|| std::env::var_os("RCE_FOREVER_NAVIGATION56_MAP"))
            .unwrap_or_else(|| "0".into());
        let runtime = load(&package, &plan, &map).unwrap();
        assert_eq!(runtime.selected_map, 0);
    }
}
