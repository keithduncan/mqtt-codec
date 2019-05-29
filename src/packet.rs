use core::{
    convert::TryFrom,
    cmp::min,
    result::Result,
};

use crate::{
    fixed_header::{self, FixedHeader},
    variable_header::{self, VariableHeader},
    payload::{self, Payload},
    status::Status,
    error::{DecodeError, EncodeError},
    codec::{Decodable, Encodable},
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct Packet<'a> {
    fixed_header: FixedHeader,
    variable_header: Option<VariableHeader<'a>>,
    payload: Option<Payload<'a>>,
}

impl<'a> Packet<'a> {
    pub fn connect(variable_header: variable_header::connect::Connect<'a>, payload: payload::connect::Connect<'a>) -> Result<Packet<'a>, EncodeError> {
        let len = u32::try_from(variable_header.encoded_len() + payload.encoded_len())?;
        Ok(Packet {
            fixed_header: FixedHeader::new(
                fixed_header::PacketType::Connect,
                0,
                len,
            ),
            variable_header: Some(variable_header::VariableHeader::Connect(variable_header)),
            payload: Some(payload::Payload::Connect(payload)),
        })
    }

    pub fn fixed_header(&self) -> &FixedHeader {
        &self.fixed_header
    }
}

impl<'a> Decodable<'a> for Packet<'a> {
    fn from_bytes(bytes: &'a [u8]) -> Result<Status<(usize, Self)>, DecodeError> {
        let (fixed_header_offset, fixed_header) = read!(FixedHeader::from_bytes, bytes, 0);

        // TODO this is only duplicated while not all types have their
        // variable header parsed.
        let (variable_header, payload) = if let Some(result) = VariableHeader::from_bytes(fixed_header.r#type(), &bytes[fixed_header_offset..]) {
            let (variable_header_offset, variable_header) = match result {
                Err(e) => return Err(e),
                Ok(Status::Partial(p)) => return Ok(Status::Partial(p)),
                Ok(Status::Complete(x)) => x,
            };
            let variable_header_consumed = variable_header_offset;

            let payload_len = fixed_header.len() as usize - variable_header_consumed;

            let available = bytes.len() - (fixed_header_offset + variable_header_offset);
            let needed = payload_len - min(available, payload_len);
            if needed > 0 {
                return Ok(Status::Partial(needed));
            }
            let payload = &bytes[fixed_header_offset+variable_header_offset..fixed_header_offset+variable_header_offset+payload_len];

            (Some(variable_header), payload)
        } else {
            let available = bytes.len() - fixed_header_offset;
            let needed = fixed_header.len() as usize - min(available, fixed_header.len() as usize);
            if needed > 0 {
                return Ok(Status::Partial(needed));
            }
            let payload = &bytes[fixed_header_offset..fixed_header_offset+fixed_header.len() as usize];

            (None, payload)
        };

        let payload = Some(payload::Payload::Bytes(payload));

        Ok(Status::Complete((fixed_header_offset + fixed_header.len() as usize, Packet {
            fixed_header,
            variable_header,
            payload,
        })))
    }
}

impl<'a> Encodable for Packet<'a> {
    fn encoded_len(&self) -> usize {
        unimplemented!()
    }

    fn to_bytes(&self, bytes: &mut [u8]) -> Result<usize, EncodeError> {
        let mut offset = 0;

        offset = {
            let o = self.fixed_header.to_bytes(&mut bytes[offset..])?;
            offset + o
        };

        if let Some(ref variable_header) = self.variable_header {
            offset = {
                let o = variable_header.to_bytes(&mut bytes[offset..])?;
                offset + o
            };
        }

        let offset = if let Some(ref payload) = self.payload {
            let o = payload.to_bytes(&mut bytes[offset..])?;
            offset + o
        } else {
            offset
        };

        Ok(offset)
    }
}
