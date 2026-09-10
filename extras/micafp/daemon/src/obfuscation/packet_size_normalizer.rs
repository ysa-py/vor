use rand::Rng;
use rand::RngCore;

pub fn pad_to_target(data: &mut Vec<u8>, target_size: usize) {
    if data.len() >= target_size { return; }
    let padding_len = target_size - data.len();
    let mut rng = rand::thread_rng();
    let mut padding = vec![0u8; padding_len];
    rng.fill_bytes(&mut padding);
    data.extend_from_slice(&padding);
}

pub fn add_tls_padding(data: &mut Vec<u8>) {
    let current_len = data.len();
    let target = select_target_size(current_len);
    if target > current_len {
        let padding_len = target - current_len;
        data.push(0x15); // TLS padding content type
        data.push(0x03); data.push(0x03);
        data.push(((padding_len - 5) >> 8) as u8);
        data.push(((padding_len - 5) & 0xFF) as u8);
        let mut rng = rand::thread_rng();
        let padding = vec![0u8; padding_len - 5];
        data.extend_from_slice(&padding);
    }
}

pub fn remove_padding(data: &[u8]) -> Vec<u8> {
    for i in (0..data.len()).rev() {
        if data[i] != 0x00 { return data[0..=i].to_vec(); }
    }
    data.to_vec()
}

fn select_target_size(current: usize) -> usize {
    let mut rng = rand::thread_rng();
    let targets = [1400, 2800, 4200, 5800];
    *targets.iter().find(|&&t| t >= current).unwrap_or(&5800)
}
