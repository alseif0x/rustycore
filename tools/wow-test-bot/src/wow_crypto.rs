//! WorldCrypt implementation using AES-128-GCM for WoW 3.4.3 (WotLK Classic)
//! Matches TrinityCore's WorldPacketCrypt exactly.
use openssl::symm::{Cipher, Crypter, Mode};

const CLIENT_MAGIC: u32 = 0x544E4C43; // "CLNT"
const SERVER_MAGIC: u32 = 0x52565253; // "SRVR"

pub struct WorldCrypt {
    key: [u8; 16],
    client_counter: u64,
    server_counter: u64,
}

impl WorldCrypt {
    /// Create a new WorldCrypt using the first 16 bytes of the session key.
    pub fn new(session_key: &[u8]) -> Self {
        let mut key = [0u8; 16];
        let len = session_key.len().min(16);
        key[..len].copy_from_slice(&session_key[..len]);
        Self {
            key,
            client_counter: 0,
            server_counter: 0,
        }
    }

    /// Create new WorldCrypt with given 16-byte key and initial counters.
    pub fn new_with_counters(key: &[u8; 16], send_counter: u32, recv_counter: u32) -> Self {
        Self {
            key: *key,
            client_counter: send_counter as u64,
            server_counter: recv_counter as u64,
        }
    }

    fn client_iv(&self) -> [u8; 12] {
        let mut iv = [0u8; 12];
        iv[0..8].copy_from_slice(&self.client_counter.to_le_bytes());
        iv[8..12].copy_from_slice(&CLIENT_MAGIC.to_le_bytes());
        iv
    }

    fn server_iv(&self) -> [u8; 12] {
        let mut iv = [0u8; 12];
        iv[0..8].copy_from_slice(&self.server_counter.to_le_bytes());
        iv[8..12].copy_from_slice(&SERVER_MAGIC.to_le_bytes());
        iv
    }

    /// Encrypt client packet (client -> server)
    pub fn encrypt_client(
        &mut self,
        plaintext: &[u8],
        _aad: &[u8],
    ) -> Result<(Vec<u8>, [u8; 12]), String> {
        let iv = self.client_iv();
        let result = aes_128_gcm_encrypt(&self.key, &iv, plaintext)?;
        self.client_counter = self.client_counter.wrapping_add(1);
        Ok(result)
    }

    /// Decrypt server packet (server -> client)
    pub fn decrypt_server(
        &mut self,
        ciphertext: &[u8],
        tag: &[u8; 12],
        _aad: &[u8],
    ) -> Result<Vec<u8>, String> {
        let iv = self.server_iv();
        let result = aes_128_gcm_decrypt(&self.key, &iv, ciphertext, tag)?;
        self.server_counter = self.server_counter.wrapping_add(1);
        Ok(result)
    }

    /// Encrypt server packet (for testing/simulation)
    pub fn encrypt_server(
        &mut self,
        plaintext: &[u8],
        _aad: &[u8],
    ) -> Result<(Vec<u8>, [u8; 12]), String> {
        let iv = self.server_iv();
        let result = aes_128_gcm_encrypt(&self.key, &iv, plaintext)?;
        self.server_counter = self.server_counter.wrapping_add(1);
        Ok(result)
    }

    /// Decrypt client packet (for testing/simulation)
    pub fn decrypt_client(
        &mut self,
        ciphertext: &[u8],
        tag: &[u8; 12],
        _aad: &[u8],
    ) -> Result<Vec<u8>, String> {
        let iv = self.client_iv();
        let result = aes_128_gcm_decrypt(&self.key, &iv, ciphertext, tag)?;
        self.client_counter = self.client_counter.wrapping_add(1);
        Ok(result)
    }

    /// Encrypt a full client packet into wire format.
    ///
    /// Wire format (TrinityCore 3.4.3 client -> server):
    ///   [size: u32 LE][tag: 12 bytes][ciphertext: size bytes]
    ///
    /// The first 2 bytes of ciphertext are the encrypted opcode, matching the
    /// 18-byte IncomingPacketHeader layout used by TrinityCore:
    ///   header = size(4) + tag(12) + encryptedOpcode(2)
    ///   body   = ciphertext[2..] (size - 2 bytes)
    pub fn encrypt_client_packet(&mut self, opcode: u16, data: &[u8]) -> Vec<u8> {
        let mut plaintext = Vec::with_capacity(2 + data.len());
        plaintext.extend_from_slice(&opcode.to_le_bytes());
        plaintext.extend_from_slice(data);

        let (ciphertext, tag) = self
            .encrypt_client(&plaintext, &[])
            .expect("Client encryption failed");

        let size = ciphertext.len() as u32;
        let mut packet = Vec::with_capacity(4 + 12 + ciphertext.len());
        packet.extend_from_slice(&size.to_le_bytes());
        packet.extend_from_slice(&tag);
        packet.extend_from_slice(&ciphertext);
        packet
    }

    /// Decrypt a full server packet from wire format.
    ///
    /// Wire format (TrinityCore 3.4.3 server -> client):
    ///   [size: u32 LE][tag: 12 bytes][ciphertext: size bytes]
    ///
    /// Returns: (opcode, payload)
    pub fn decrypt_server_packet(&mut self, data: &[u8]) -> (u16, Vec<u8>) {
        if data.len() < 16 {
            panic!("Packet too short for header: {} bytes", data.len());
        }

        let size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        if data.len() < 16 + size {
            panic!(
                "Packet too short: expected {} bytes, got {}",
                16 + size,
                data.len()
            );
        }

        let mut tag = [0u8; 12];
        tag.copy_from_slice(&data[4..16]);
        let ciphertext = &data[16..16 + size];

        let plaintext = self
            .decrypt_server(ciphertext, &tag, &[])
            .expect("Server decryption failed");

        if plaintext.len() < 2 {
            panic!(
                "Decrypted payload too short for opcode: {} bytes",
                plaintext.len()
            );
        }

        let opcode = u16::from_le_bytes([plaintext[0], plaintext[1]]);
        let payload = plaintext[2..].to_vec();
        (opcode, payload)
    }
}

fn aes_128_gcm_encrypt(
    key: &[u8; 16],
    iv: &[u8; 12],
    plaintext: &[u8],
) -> Result<(Vec<u8>, [u8; 12]), String> {
    let cipher = Cipher::aes_128_gcm();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(iv))
        .map_err(|e| format!("Crypter init failed: {}", e))?;
    let mut ciphertext = vec![0u8; plaintext.len() + cipher.block_size()];
    let mut count = crypter
        .update(plaintext, &mut ciphertext)
        .map_err(|e| format!("Encrypt update failed: {}", e))?;
    count += crypter
        .finalize(&mut ciphertext[count..])
        .map_err(|e| format!("Encrypt finalize failed: {}", e))?;
    ciphertext.truncate(count);

    let mut tag = [0u8; 12];
    crypter
        .get_tag(&mut tag)
        .map_err(|e| format!("Get tag failed: {}", e))?;

    Ok((ciphertext, tag))
}

fn aes_128_gcm_decrypt(
    key: &[u8; 16],
    iv: &[u8; 12],
    ciphertext: &[u8],
    tag: &[u8; 12],
) -> Result<Vec<u8>, String> {
    let cipher = Cipher::aes_128_gcm();
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(iv))
        .map_err(|e| format!("Crypter init failed: {}", e))?;
    crypter
        .set_tag(tag)
        .map_err(|e| format!("Set tag failed: {}", e))?;
    let mut plaintext = vec![0u8; ciphertext.len() + cipher.block_size()];
    let mut count = crypter
        .update(ciphertext, &mut plaintext)
        .map_err(|e| format!("Decrypt update failed: {}", e))?;
    count += crypter
        .finalize(&mut plaintext[count..])
        .map_err(|e| format!("Decrypt finalize failed: {}", e))?;
    plaintext.truncate(count);
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypt_roundtrip() {
        let key = [0xABu8; 16];
        let mut client_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
        let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);

        let plaintext = b"Hello, World!";
        let (ciphertext, tag) = client_crypt.encrypt_client(plaintext, &[]).unwrap();
        // Client and server traffic use distinct nonce domains. This fixture
        // encrypts client-to-server traffic, so decrypt it in that direction.
        let decrypted = server_crypt.decrypt_client(&ciphertext, &tag, &[]).unwrap();

        assert_eq!(plaintext[..], decrypted[..]);
    }

    #[test]
    fn test_encrypt_client_packet() {
        let key = [0xCDu8; 16];
        let mut crypt = WorldCrypt::new_with_counters(&key, 0, 0);

        let data = b"test payload";
        let packet = crypt.encrypt_client_packet(0x1234, data);

        // Should be: 4 bytes size + 12 bytes tag + size bytes ciphertext
        let size = u32::from_le_bytes([packet[0], packet[1], packet[2], packet[3]]) as usize;
        assert_eq!(packet.len(), 16 + size);
    }

    #[test]
    fn test_decrypt_server_packet() {
        let key = [0xEFu8; 16];
        let mut server_crypt = WorldCrypt::new_with_counters(&key, 0, 0);
        let mut client_crypt = WorldCrypt::new_with_counters(&key, 0, 0);

        let opcode = 0x5678u16;
        let payload = b"server response";

        // Simulate server sending a packet
        let mut plaintext = Vec::new();
        plaintext.extend_from_slice(&opcode.to_le_bytes());
        plaintext.extend_from_slice(payload);

        let (ciphertext, tag) = server_crypt.encrypt_server(&plaintext, &[]).unwrap();
        let size = ciphertext.len() as u32;

        let mut packet = Vec::new();
        packet.extend_from_slice(&size.to_le_bytes());
        packet.extend_from_slice(&tag);
        packet.extend_from_slice(&ciphertext);

        let (decrypted_opcode, decrypted_payload) = client_crypt.decrypt_server_packet(&packet);

        assert_eq!(decrypted_opcode, opcode);
        assert_eq!(decrypted_payload[..], payload[..]);
    }
}
