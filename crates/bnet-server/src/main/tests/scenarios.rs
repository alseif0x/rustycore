//! Battle.net server entry regressions.
//!
//! Moved out of main.rs under #685; every test is unchanged.

use super::*;

#[test]
fn bnet_config_resolution_prefers_lowercase_cpp_name() {
    let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
    let root = unique_temp_dir("bnet_config_resolution");
    let lower = root.join("bnetserver.conf");
    let legacy = root.join("BNetServer.conf");

    fs::write(&lower, "BattlenetPort = 1119\n").expect("write lower failed");
    fs::write(&legacy, "BattlenetPort = 2222\n").expect("write legacy failed");

    let report =
        load_bnet_config_from(&lower, &root.join("bnetserver.conf.d")).expect("config should load");

    assert_eq!(report.candidate_index, 0);
    assert_eq!(wow_config::get_value::<u16>("BattlenetPort"), Some(1119));

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn encrypted_pkcs8_private_key_uses_private_key_password_like_cpp() {
    let der = decrypt_pkcs8_private_key_pem_like_cpp(ENCRYPTED_PKCS8_TEST_KEY, "secret")
        .expect("encrypted key should decrypt");
    assert!(matches!(der.first(), Some(0x30)));
    assert!(rustls::pki_types::PrivateKeyDer::try_from(der).is_ok());
}

#[test]
fn encrypted_pkcs8_private_key_rejects_wrong_password_like_cpp() {
    let error = decrypt_pkcs8_private_key_pem_like_cpp(ENCRYPTED_PKCS8_TEST_KEY, "wrong")
        .expect_err("wrong password should fail");
    assert!(error.to_string().contains("Failed to decrypt"));
}

#[test]
fn private_key_password_requires_encrypted_pkcs8_pem() {
    let root = unique_temp_dir("bnet_tls_password_requires_encrypted_pkcs8");
    let key_path = root.join("key.pem");
    fs::write(
        &key_path,
        r#"-----BEGIN PRIVATE KEY-----
MAoCAQAwBQYDK2Vw
-----END PRIVATE KEY-----
"#,
    )
    .expect("write key failed");

    let error = load_private_key_like_cpp(key_path.to_str().unwrap(), Some("secret"))
        .expect_err("non-encrypted key should fail when password is configured");
    assert!(format!("{error:#}").contains("not an encrypted PKCS#8"));

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn bnet_config_loads_cpp_section_and_tls_paths_like_cpp() {
    let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
    let root = unique_temp_dir("bnet_config_cpp_section");
    let lower = root.join("bnetserver.conf");

    fs::write(
        &lower,
        r#"
[bnetserver]
BattlenetPort = 1119
LoginREST.Port = 8081
CertificatesFile = "/tmp/bnetserver.cert.pem"
PrivateKeyFile = "/tmp/bnetserver.key.pem"
PrivateKeyPassword = "secret"
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
"#,
    )
    .expect("write lower failed");

    load_bnet_config_from(&lower, &root.join("bnetserver.conf.d")).expect("config should load");

    assert_eq!(
        wow_config::get_string_default("CertificatesFile", ""),
        "/tmp/bnetserver.cert.pem"
    );
    assert_eq!(
        wow_config::get_string_default("PrivateKeyFile", ""),
        "/tmp/bnetserver.key.pem"
    );
    assert_eq!(
        wow_config::get_string_default("PrivateKeyPassword", ""),
        "secret"
    );
    let info = wow_config::get_database_info_default(
        "Login",
        wow_config::DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
    );
    assert_eq!(info.database, "auth");

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn bnet_config_resolution_uses_dist_fallback_for_explicit_config_like_cpp() {
    let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
    let root = unique_temp_dir("bnet_config_dist_fallback");
    let config = root.join("custom-bnet.conf");
    let dist = root.join("custom-bnet.conf.dist");

    fs::write(&dist, "BattlenetPort = 3333\n").expect("write dist failed");

    let report = load_bnet_config_from(&config, &root.join("bnetserver.conf.d"))
        .expect("dist config should load");

    assert_eq!(report.candidate_index, 1);
    assert_eq!(wow_config::get_value::<u16>("BattlenetPort"), Some(3333));

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn bnet_cli_parser_accepts_cpp_aliases_and_ignores_unknowns_like_cpp() {
    let cli = BnetCliLikeCpp::parse_from([
        "-c".to_string(),
        "custom.conf".to_string(),
        "-cd".to_string(),
        "custom.conf.d".to_string(),
        "--unknown".to_string(),
    ]);

    assert_eq!(cli.config_file, PathBuf::from("custom.conf"));
    assert_eq!(cli.config_dir, PathBuf::from("custom.conf.d"));
    assert!(!cli.show_help);
    assert!(!cli.show_version);
}

#[test]
fn bnet_cli_parser_accepts_long_equals_and_early_exit_flags_like_cpp() {
    let cli = BnetCliLikeCpp::parse_from([
        "--config=custom.conf".to_string(),
        "--config-dir=custom.conf.d".to_string(),
        "--help".to_string(),
        "--version".to_string(),
    ]);

    assert_eq!(cli.config_file, PathBuf::from("custom.conf"));
    assert_eq!(cli.config_dir, PathBuf::from("custom.conf.d"));
    assert!(cli.show_help);
    assert!(cli.show_version);
    assert!(!bnet_cli_help_like_cpp().contains(&["--update-databases", "-only"].concat()));
}

#[test]
fn db_keep_alive_interval_is_configured_in_minutes_like_cpp() {
    assert_eq!(
        db_keep_alive_interval_duration_like_cpp(30),
        std::time::Duration::from_secs(30 * 60)
    );
    assert_eq!(
        db_keep_alive_interval_duration_like_cpp(1),
        std::time::Duration::from_secs(60)
    );
}

#[test]
fn bnet_startup_only_validates_schema_before_runtime_database_writes() {
    let source = include_str!("../../main.rs");
    assert!(!source.contains(&["populate_typed_", "database_like_cpp"].concat()));
    assert!(!source.contains(&["update_typed_", "database_like_cpp"].concat()));
    assert!(!source.contains(&["update-databases", "-only"].concat()));
    let validation = source.find("validate_runtime_schema(").unwrap();
    let first_runtime_write = source
        .find("migrate_legacy_password_hashes_like_cpp")
        .unwrap();
    assert!(validation < first_runtime_write);
}

#[test]
fn bnet_thread_config_is_observed_but_not_applied_to_acceptors_like_cpp() {
    let config = bnet_thread_config_from_values_like_cpp(4, 8);

    assert_eq!(config.network_threads, 4);
    assert_eq!(config.login_rest_thread_count, 8);
    assert!(!config.applies_to_bnet_acceptors);
}

#[test]
fn listener_task_clean_exit_is_fatal_like_cpp_network_failure() {
    let error = listener_task_exit_like_cpp("REST", Ok(())).expect_err("listener exit must fail");

    assert!(
        error
            .to_string()
            .contains("REST listener stopped unexpectedly")
    );
}

#[test]
fn login_rest_resolution_selects_ipv4_endpoint_like_cpp() {
    let endpoints = [
        "[::1]:8081".parse::<SocketAddr>().unwrap(),
        "192.0.2.10:8081".parse::<SocketAddr>().unwrap(),
    ];

    assert_eq!(
        first_ipv4_address_like_cpp(endpoints),
        Some(Ipv4Addr::new(192, 0, 2, 10))
    );
    assert_eq!(
        first_ipv4_address_like_cpp(["[::1]:8081".parse::<SocketAddr>().unwrap()]),
        None
    );
}

#[test]
fn create_pid_file_writes_current_process_id_like_cpp() {
    let root = unique_temp_dir("pid_file");
    fs::create_dir_all(&root).expect("create temp dir failed");
    let pid_file = root.join("bnetserver.pid");

    let pid = create_pid_file_like_cpp(&pid_file).expect("pid file should be created");

    assert_eq!(pid, std::process::id());
    assert_eq!(
        fs::read_to_string(&pid_file).expect("pid file should be readable"),
        std::process::id().to_string()
    );

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn bnet_full_version_contains_package_version_like_cpp_banner() {
    let version = bnet_full_version_like_cpp();
    assert!(version.contains("RustyCore BNet Server"));
    assert!(version.contains(env!("CARGO_PKG_VERSION")));
}
