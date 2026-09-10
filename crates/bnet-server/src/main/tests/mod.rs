//! Battle.net server entry regressions.
//!
//! Separated from main.rs under #685.

use crate::*;

use super::{
    BnetCliLikeCpp, bnet_cli_help_like_cpp, bnet_full_version_like_cpp,
    bnet_thread_config_from_values_like_cpp, create_pid_file_like_cpp,
    db_keep_alive_interval_duration_like_cpp, decrypt_pkcs8_private_key_pem_like_cpp,
    first_ipv4_address_like_cpp, listener_task_exit_like_cpp, load_bnet_config_from,
    load_private_key_like_cpp,
};
use std::env;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Mutex;

static CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());
const ENCRYPTED_PKCS8_TEST_KEY: &str = r#"-----BEGIN ENCRYPTED PRIVATE KEY-----
MIIC3TBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQIUyBCum5/y54CAggA
MAwGCCqGSIb3DQIJBQAwHQYJYIZIAWUDBAEqBBC/XmvFo8zjwfieHYC70YDrBIIC
gOZC8gzx2anQD8lvyzVhWKpupCrl0KcOnF78xdY5tka278fTNZHZaiHgG3gN/2BA
XoZUcoggibt4R5Cv3gVOl+XJazTZVz905nabPKY2DX0mvlkC6QG1eD/QIQSJf3xy
JIJVz4/EMMpEfoRGzAopvYDT5KOoicWMyOT3wRGjFhQ7pkS8K1gknfOS/nJ2MReo
huAqvQWzv3QG1k0ywnBNfqLVJIYncAEdJ0EbveFK+iYD3/Ie2RjCRIPVUUx7mQZN
GdlQYWmJC0XD3YSJlCLwiKDS/4VFMnWZVIvo9Fja+0kVtRnq/Lh5rSXALRP65S+q
rY84agGD8YvnN1DjC0K/4chisdd4bTBr0U1G6gX6yieNsBzS/1LRIa3NpHvPL6Ta
atVzEs0R0Rnn2zhdBpixOBvFLOgge+NOPx5twOQUIBlCwgtHxBIFRBcz+9Au+mPH
bky5a18a2uwIR03v8DKCPX4zZnWsTy5IERcvu+y0m+D9bzNf5p9bob/CQPxx3kUB
dK42FcCpu0+mnP+SImsdNVufD9qCgmoxgM78kn2mInzdPs7y3otDwc4dfCCxSjyV
bCb2P11mgDUY1gODqvAmD7DEyghiZtUusCKcphBHFw+vobReIKXFAK9a3xrrux2Y
wK0J/RFBYJEw9aYFA5iHRQVVmzCyKro+EaQSrN9/Xi2n3YzRqMY/pduQ6qJ4xA5Q
DkpzLQyZJUrrBCu3ErEKKgJDB4zUoeA2Zx1QI0NffLwF4O0C+2jtVROs887b0kTx
7e6w3smBjkBREUiXdlDW+PYUpIUDFAqjWF8rxk3tg9H+9qeSz3a+vEnuT10pkq3A
5llBUo/cIM8wieR7BJNlnVs=
-----END ENCRYPTED PRIVATE KEY-----"#;

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rustycore_bnet_server_{name}_{}",
        std::process::id()
    ));

    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temp dir failed");
    path
}
mod scenarios;
