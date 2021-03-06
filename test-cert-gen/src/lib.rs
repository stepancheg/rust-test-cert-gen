//! Utilities to generate keys for tests.
//!
//! This is copy-paste from tokio-tls.

use std::fs;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use std::process::Stdio;
use std::ptr;
use std::sync::Once;

/// Client certificate
pub struct ClientKeys {
    pub cert_der: Vec<u8>,
}

/// Server keys
pub struct ServerKeys {
    /// Certificate and key
    pub pkcs12: Vec<u8>,
    /// Password from `pkcs12`
    pub pkcs12_password: String,

    /// The same in PEM format
    pub pem: Vec<u8>,
}

/// Client and server keys
pub struct Keys {
    /// Client keys
    pub client: ClientKeys,
    /// Server keys
    pub server: ServerKeys,
}

fn gen_keys() -> Keys {
    let temp_dir = tempdir::TempDir::new("rust-test-cert-gen").unwrap();

    let keyfile = temp_dir.path().join("test.key");
    let certfile = temp_dir.path().join("test.crt");
    let config = temp_dir.path().join("openssl.config");

    fs::write(
        &config,
        b"\
                [req]\n\
                distinguished_name=dn\n\
                [dn]\n\
                CN=localhost\n\
                [ext]\n\
                basicConstraints=CA:FALSE,pathlen:0\n\
                subjectAltName = @alt_names\n\
                extendedKeyUsage=serverAuth,clientAuth\n\
                [alt_names]\n\
                DNS.1 = localhost\n\
            ",
    )
    .unwrap();

    let subj = "/C=US/ST=Denial/L=Sprintfield/O=Dis/CN=localhost";
    let output = Command::new("openssl")
        .arg("req")
        .arg("-nodes")
        .arg("-x509")
        .arg("-newkey")
        .arg("rsa:2048")
        .arg("-config")
        .arg(&config)
        .arg("-extensions")
        .arg("ext")
        .arg("-subj")
        .arg(subj)
        .arg("-keyout")
        .arg(&keyfile)
        .arg("-out")
        .arg(&certfile)
        .arg("-days")
        .arg("1")
        .output()
        .unwrap();
    assert!(output.status.success());

    let crtout = Command::new("openssl")
        .arg("x509")
        .arg("-outform")
        .arg("der")
        .arg("-in")
        .arg(&certfile)
        .output()
        .unwrap();
    assert!(crtout.status.success());

    let pkcs12out = Command::new("openssl")
        .arg("pkcs12")
        .arg("-export")
        .arg("-nodes")
        .arg("-inkey")
        .arg(&keyfile)
        .arg("-in")
        .arg(&certfile)
        .arg("-password")
        .arg("pass:foobar")
        .output()
        .unwrap();
    assert!(pkcs12out.status.success());

    let pem = pkcs12_to_pem(&pkcs12out.stdout, "foobar");

    Keys {
        client: ClientKeys {
            cert_der: crtout.stdout,
        },
        server: ServerKeys {
            pem,
            pkcs12: pkcs12out.stdout,
            pkcs12_password: "foobar".to_owned(),
        },
    }
}

/// Generate keys
pub fn keys() -> &'static Keys {
    static INIT: Once = Once::new();
    static mut KEYS: *mut Keys = ptr::null_mut();

    INIT.call_once(|| unsafe {
        KEYS = Box::into_raw(Box::new(gen_keys()));
    });
    unsafe { &*KEYS }
}

fn pkcs12_to_pem(pkcs12: &[u8], passin: &str) -> Vec<u8> {
    let command = Command::new("openssl")
        .arg("pkcs12")
        .arg("-passin")
        .arg(&format!("pass:{}", passin))
        .arg("-nodes")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    command.stdin.unwrap().write_all(pkcs12).unwrap();

    let mut pem = Vec::new();
    command.stdout.unwrap().read_to_end(&mut pem).unwrap();

    pem
}

#[cfg(test)]
mod test {
    #[test]
    fn test() {
        // just check it does something
        super::keys();
    }
}
