use super::util::DEFAULT_MIRI_TARGET_RELPATH;
use crate::base_crate::BaseCrateType;
use crate::commands::build::do_cached_build;
use crate::{base_crate::new_base_crate, error::Errno, error_msg};
use crate::{
    cli::TestArgs,
    config::{Config, scheme::ActionChoice},
    util::{get_current_crates, get_target_directory},
};
use std::fs;
use std::path::PathBuf;

pub fn execute_miri_command(config: &Config, args: &TestArgs) {
    let crates = get_current_crates();
    for krate in crates {
        std::env::set_current_dir(&krate.path).unwrap();
        miri_current_crate(config, args, ActionChoice::Miri);
    }
}

pub fn execute_miri_debugger_command(config: &Config, args: &TestArgs) {
    let crates = get_current_crates();
    for krate in crates {
        std::env::set_current_dir(&krate.path).unwrap();
        _ = fs::remove_dir_all("target");
        let paths = miri_current_crate(config, args, ActionChoice::MiriDebugger);
        paths.copy_analysis_json();
        paths.do_cached_build(config, ActionChoice::Miri);
    }
}

struct CrateDirectoryPaths {
    default_bundle_directory: PathBuf,
    osdk_output_directory: PathBuf,
    cargo_target_directory: PathBuf,
    target_crate_dir: PathBuf,
}

impl CrateDirectoryPaths {
    fn do_cached_build(&self, config: &Config, action: ActionChoice) {
        std::env::set_current_dir(&self.target_crate_dir).unwrap();
        do_cached_build(
            &self.default_bundle_directory,
            &self.osdk_output_directory,
            &self.cargo_target_directory,
            config,
            action,
            &["--cfg=ktest", "--cfg=miri"],
        );
    }

    fn copy_analysis_json(&self) {
        let src = self.target_crate_dir.join("target").join("analysis.json");
        assert!(src.exists());
        // let dest = self.target_crate_dir.join("analysis.json");
        let dest = self.cargo_target_directory.join("analysis.json");
        fs::copy(src, dest).unwrap();
    }
}

/// Miri or KMiriHelper doesn't generate full binary artifacts.
/// The returned path refers to the base crate, usually being `$proj/target/osdk-miri/${proj}-base`.
fn miri_current_crate(
    config: &Config,
    args: &TestArgs,
    action: ActionChoice,
) -> CrateDirectoryPaths {
    let current_crate = &get_current_crates()[0];
    let cargo_target_directory = get_target_directory();
    let osdk_output_directory = cargo_target_directory.join(DEFAULT_MIRI_TARGET_RELPATH);
    let target_crate_dir = osdk_output_directory.join(&current_crate.name);
    // A special case is that we use OSDK to test the OSDK test runner crate
    // itself. We check it by name.
    let runner_self_test = if current_crate.name == "osdk-test-kernel" {
        if matches!(option_env!("OSDK_LOCAL_DEV"), Some("1")) {
            true
        } else {
            error_msg!("The tested crate name collides with the OSDK test runner crate");
            std::process::exit(Errno::BadCrateName as _);
        }
    } else {
        false
    };
    let target_crate_dir = new_base_crate(
        BaseCrateType::Miri,
        &target_crate_dir,
        &current_crate.name,
        &current_crate.path,
        !runner_self_test,
    );
    let main_rs_path = target_crate_dir.join("src").join("main.rs");
    let ktest_test_whitelist = if args.test_name.is_empty() {
        r#"None"#.to_string()
    } else {
        format!(r#"Some(&{:?})"#, args.test_name)
    };
    let ktest_crate_whitelist = vec![current_crate.name.clone()];
    // if let Some(name) = &args.test_name {
    //     ktest_crate_whitelist.push(name.clone());
    // }
    // Append the ktest static variable and the runner reference to the
    // `main.rs` file.
    let ktest_main_rs = format!(
        r#"
{}
#[unsafe(no_mangle)]
pub static KTEST_TEST_WHITELIST: Option<&[&str]> = {};
#[unsafe(no_mangle)]
pub static KTEST_CRATE_WHITELIST: Option<&[&str]> = Some(&{:#?});
"#,
        if runner_self_test {
            ""
        } else {
            "extern crate osdk_test_kernel;"
        },
        ktest_test_whitelist,
        ktest_crate_whitelist,
    );
    let mut main_rs_content = fs::read_to_string(&main_rs_path).unwrap();
    main_rs_content.push_str(&ktest_main_rs);
    fs::write(&main_rs_path, main_rs_content).unwrap();
    // Build the kernel with the given base crate
    let target_name = get_current_crates()[0].name.clone();
    let default_bundle_directory = osdk_output_directory.join(target_name);

    let paths = CrateDirectoryPaths {
        default_bundle_directory,
        osdk_output_directory,
        cargo_target_directory,
        target_crate_dir,
    };
    paths.do_cached_build(config, action);
    paths
}
