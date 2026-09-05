use conformance_contract::{Fault, Result, State};

pub const MAX_CHECKPOINTS: usize = 8;
pub const HEADER_BYTES: usize = 15;
pub const STATE_LIMIT: usize = HEADER_BYTES + MAX_CHECKPOINTS;

/// An owned, variable-length set plus history. Deliberately not Clone: the host
/// owns its canonical instance; invocations use decoded, short-lived projections.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct ExpeditionState {
    pub resets: u32,
    pub accepted_total: u64,
    pub checkpoints: Vec<u8>,
}

impl State for ExpeditionState {
    const SCHEMA: u32 = 1;

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_BYTES + self.checkpoints.len());
        bytes.extend_from_slice(&[b'E', 1]);
        bytes.extend_from_slice(&self.resets.to_le_bytes());
        bytes.extend_from_slice(&self.accepted_total.to_le_bytes());
        bytes.push(self.checkpoints.len() as u8);
        bytes.extend_from_slice(&self.checkpoints);
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > STATE_LIMIT {
            return Err(Fault::Limit);
        }
        if bytes.len() < HEADER_BYTES || bytes[..2] != [b'E', 1] {
            return Err(Fault::Invalid);
        }
        let count = usize::from(bytes[14]);
        if count > MAX_CHECKPOINTS || bytes.len() != HEADER_BYTES + count {
            return Err(Fault::Invalid);
        }
        let checkpoints = bytes[HEADER_BYTES..].to_vec();
        if checkpoints.iter().any(|id| !(1..=31).contains(id))
            || checkpoints.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(Fault::Invalid);
        }
        let resets = u32::from_le_bytes(bytes[2..6].try_into().map_err(|_| Fault::Invalid)?);
        let accepted_total =
            u64::from_le_bytes(bytes[6..14].try_into().map_err(|_| Fault::Invalid)?);
        if accepted_total < count as u64 {
            return Err(Fault::Invalid);
        }
        Ok(Self {
            resets,
            accepted_total,
            checkpoints,
        })
    }
}
