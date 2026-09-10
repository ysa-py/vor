const STEGO_MAGIC: [u8; 2] = [0x5A, 0x53]; // "ZS"
const STEGO_VERSION: u8 = 0x01;

pub struct StegoHeader {
    pub session_id: u32,
    pub payload: Vec<u8>,
}

impl StegoHeader {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + self.payload.len());
        out.extend_from_slice(&STEGO_MAGIC);
        out.push(STEGO_VERSION);
        out.extend_from_slice(&self.session_id.to_be_bytes());
        out.push(((self.payload.len() >> 8) & 0xFF) as u8);
        out.push((self.payload.len() & 0xFF) as u8);
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 9 { return None; }
        if data[0..2] != STEGO_MAGIC { return None; }
        if data[2] != STEGO_VERSION { return None; }
        let session_id = u32::from_be_bytes(data[3..7].try_into().ok()?);
        let payload_len = ((data[7] as usize) << 8) | (data[8] as usize);
        if data.len() < 9 + payload_len { return None; }
        Some(Self { session_id, payload: data[9..9+payload_len].to_vec() })
    }

    pub fn encode_in_session_ticket(data: &[u8]) -> Vec<u8> {
        let mut ticket = Vec::with_capacity(4 + data.len());
        ticket.extend_from_slice(&(data.len() as u16).to_be_bytes());
        ticket.push(0x00); ticket.push(0x23); // Session ticket type
        ticket.extend_from_slice(data);
        ticket
    }

    pub fn decode_from_session_ticket(ticket: &[u8]) -> Option<Vec<u8>> {
        if ticket.len() < 4 { return None; }
        let len = u16::from_be_bytes(ticket[0..2].try_into().ok()?) as usize;
        if ticket.len() < 4 + len { return None; }
        Some(ticket[4..4+len].to_vec())
    }

    pub fn encode_in_cert_status(data: &[u8]) -> Vec<u8> {
        let mut status = Vec::with_capacity(6 + data.len());
        status.push(0x01); // OCSP type
        status.extend_from_slice(&(data.len() as u32).to_be_bytes());
        status.extend_from_slice(data);
        status
    }

    pub fn decode_from_cert_status(status: &[u8]) -> Option<Vec<u8>> {
        if status.len() < 5 || status[0] != 0x01 { return None; }
        let len = u32::from_be_bytes(status[1..5].try_into().ok()?) as usize;
        if status.len() < 5 + len { return None; }
        Some(status[5..5+len].to_vec())
    }

    pub fn encode_in_ocsp_stapling(data: &[u8]) -> Vec<u8> {
        let mut response = Vec::with_capacity(4 + data.len());
        response.extend_from_slice(&(data.len() as u32).to_be_bytes());
        response.extend_from_slice(data);
        response
    }

    pub fn decode_from_ocsp(response: &[u8]) -> Option<Vec<u8>> {
        if response.len() < 4 { return None; }
        let len = u32::from_be_bytes(response[0..4].try_into().ok()?) as usize;
        if response.len() < 4 + len { return None; }
        Some(response[4..4+len].to_vec())
    }
}
