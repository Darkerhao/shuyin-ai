use std::io::{self, Read, Write};

pub const MAX_INBOUND_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_OUTBOUND_BYTES: usize = 1024 * 1024;

pub fn read_frame(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; 4];
    match reader.read(&mut header[..1])? {
        0 => return Ok(None),
        1 => {}
        _ => unreachable!(),
    }
    reader.read_exact(&mut header[1..])?;
    let length = u32::from_le_bytes(header) as usize;
    if length == 0 || length > MAX_INBOUND_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Native Messaging frame length {length} is outside the supported range"),
        ));
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload)?;
    Ok(Some(payload))
}

pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    if payload.is_empty() || payload.len() > MAX_OUTBOUND_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Native Messaging response length {} is outside the supported range",
                payload.len()
            ),
        ));
    }
    let length = u32::try_from(payload.len()).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Native Messaging response is too large",
        )
    })?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trips_a_utf8_message_with_little_endian_length() {
        let payload = "{\"动作\":\"轮询\"}".as_bytes();
        let mut encoded = Vec::new();
        write_frame(&mut encoded, payload).unwrap();
        assert_eq!(&encoded[..4], &(payload.len() as u32).to_le_bytes());
        let mut cursor = Cursor::new(encoded);
        assert_eq!(read_frame(&mut cursor).unwrap().as_deref(), Some(payload));
        assert!(read_frame(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn rejects_empty_oversized_and_truncated_frames() {
        let mut empty = Cursor::new(0u32.to_le_bytes().to_vec());
        assert!(read_frame(&mut empty).is_err());

        let mut oversized = Cursor::new(((MAX_INBOUND_BYTES + 1) as u32).to_le_bytes().to_vec());
        assert!(read_frame(&mut oversized).is_err());

        let mut truncated = Cursor::new(vec![5, 0, 0, 0, b'{']);
        assert!(read_frame(&mut truncated).is_err());
        assert!(write_frame(&mut Vec::new(), &[]).is_err());
        assert!(write_frame(&mut Vec::new(), &vec![0; MAX_OUTBOUND_BYTES + 1]).is_err());
    }
}
