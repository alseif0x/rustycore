//! Explicit private QA observation, not character admission or a packet handler.
//! Each explicit opt-in admits exactly one post-authenticated character request:
//! CreateCharacter or CheckCharacterNameAvailability. Never auth or TACT data.
use anyhow::{Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

const OPCODE: u32 = 0x440070;
const MAX_PAYLOAD: usize = 64 * 1024;

pub(super) struct CharacterCapture {
    path: PathBuf,
    name_availability: bool,
}

impl CharacterCapture {
    pub(super) fn prepare(root: &Path, path: &Path) -> Result<Self> {
        Self::prepare_kind(root, path, false)
    }

    pub(super) fn prepare_name(root: &Path, path: &Path) -> Result<Self> {
        Self::prepare_kind(root, path, true)
    }

    fn prepare_kind(root: &Path, path: &Path, name_availability: bool) -> Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("capture parent"))?
            .canonicalize()?;
        ensure!(
            parent.starts_with(root.canonicalize()?),
            "capture outside isolated runtime"
        );
        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("capture filename"))?;
        let path = parent.join(name);
        // symlink_metadata also detects dangling final symlinks. create_new at
        // write time enforces the same no-overwrite rule without a TOCTOU gap.
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => anyhow::bail!("capture output already exists or cannot be inspected"),
        }
        Ok(Self {
            path,
            name_availability,
        })
    }

    pub(super) async fn record(&self, opcode: u32, payload: &[u8]) -> Result<()> {
        let expected = if self.name_availability {
            0x440071
        } else {
            OPCODE
        };
        if opcode != expected {
            return Ok(());
        }
        // Name request: sequence + two bit-packed length bytes + <=63 bytes
        // per string. Keep the original Create bound unchanged.
        let bound = if self.name_availability {
            132
        } else {
            MAX_PAYLOAD
        };
        ensure!(payload.len() <= bound, "capture payload limit");
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&self.path).await?;
        // FCR1 | build LE32 | opcode LE32 | payload length LE32 | payload.
        // Names/customizations are private; never log the content or pathname.
        let mut header = Vec::from(if self.name_availability {
            *b"FNR1"
        } else {
            *b"FCR1"
        });
        header.extend_from_slice(&70170u32.to_le_bytes());
        header.extend_from_slice(&expected.to_le_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        file.write_all(&header).await?;
        file.write_all(payload).await?;
        file.sync_all().await?;
        println!("Private native character request recorded; no admission/save/success inferred.");
        Ok(())
    }

    pub(super) fn accepts(&self, opcode: u32) -> bool {
        opcode
            == if self.name_availability {
                0x440071
            } else {
                OPCODE
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Root(PathBuf);
    impl Root {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forever-create-capture-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Root {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[tokio::test]
    async fn captures_only_create_privately_without_overwrite() {
        let root = Root::new();
        let path = root.0.join("request.bin");
        let capture = CharacterCapture::prepare(&root.0, &path).unwrap();
        for opcode in [0x440010, 0x440011, 0x440014, 0x450006, 0x450001] {
            capture
                .record(opcode, b"synthetic-not-captured")
                .await
                .unwrap();
        }
        assert!(!path.exists());
        capture.record(OPCODE, b"synthetic-create").await.unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"FCR1");
        assert_eq!(&bytes[4..8], &70170u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &OPCODE.to_le_bytes());
        assert_eq!(&bytes[12..16], &16u32.to_le_bytes());
        assert_eq!(&bytes[16..], b"synthetic-create");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(capture.record(OPCODE, b"replacement").await.is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(CharacterCapture::prepare(&root.0, &path).is_err());
    }

    #[tokio::test]
    async fn rejects_escape_oversize_and_final_symlink() {
        let root = Root::new();
        let outside = Root::new();
        assert!(CharacterCapture::prepare(&root.0, &outside.0.join("out.bin")).is_err());
        let path = root.0.join("request.bin");
        let capture = CharacterCapture::prepare(&root.0, &path).unwrap();
        assert!(
            capture
                .record(OPCODE, &vec![0; MAX_PAYLOAD + 1])
                .await
                .is_err()
        );
        assert!(!path.exists());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.0.join("missing.bin"), &path).unwrap();
            assert!(CharacterCapture::prepare(&root.0, &path).is_err());
            assert!(capture.record(OPCODE, b"synthetic").await.is_err());
            assert!(!root.0.join("missing.bin").exists());
        }
    }

    #[tokio::test]
    async fn name_opt_in_is_separate_bounded_private_and_non_overwriting() {
        let root = Root::new();
        let path = root.0.join("name.bin");
        let capture = CharacterCapture::prepare_name(&root.0, &path).unwrap();
        for opcode in [OPCODE, 0x440010, 0x450001, 0x450006] {
            capture.record(opcode, b"excluded").await.unwrap();
        }
        assert!(!path.exists());
        assert!(capture.record(0x440071, &[0; 133]).await.is_err());
        assert!(!path.exists());
        capture.record(0x440071, &[0; 132]).await.unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"FNR1");
        assert_eq!(&bytes[4..8], &70170u32.to_le_bytes());
        assert_eq!(&bytes[8..12], &0x440071u32.to_le_bytes());
        assert_eq!(&bytes[12..16], &132u32.to_le_bytes());
        assert_eq!(bytes.len(), 148);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(capture.record(0x440071, b"replacement").await.is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(CharacterCapture::prepare_name(&root.0, &path).is_err());
        let outside = Root::new();
        assert!(CharacterCapture::prepare_name(&root.0, &outside.0.join("out.bin")).is_err());
    }
}
