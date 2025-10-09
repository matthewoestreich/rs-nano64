pub struct Nano64Encrypted {
  iv_length: u64,
  payload_length: u64,
}

impl Default for Nano64Encrypted {
  fn default() -> Self {
      let iv_length = 12;
      Self {
        iv_length,
        payload_length: iv_length + 8 + 16,
      }
  }
}

impl Nano64Encrypted {
  
}