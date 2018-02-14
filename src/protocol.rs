use std::io;
use serde_json as json;
use byteorder::{NetworkEndian , ByteOrder};
use bytes::{BytesMut, BufMut};
use tokio_io::codec::{Encoder, Decoder};

const PREFIX: &[u8] = b"ACTIX/1.0\r\n";


/// Client request
#[derive(Serialize, Deserialize, Debug, Message)]
#[serde(tag="cmd", content="data")]
pub enum Request {
    Handshake(String),
    Ping,
    Pong,
    /// Message(msg_id, type_id, ver, payload)
    Message(String, String, String, String),
}

/// Server response
#[derive(Serialize, Deserialize, Debug, Message)]
#[serde(tag="cmd", content="data")]
pub enum Response {
    Handshake,
    Ping,
    Pong,
    /// Response(msg_id, payload)
    Response(String, String),
}

/// Codec for Client -> Server transport
pub struct NetworkServerCodec {
    prefix: bool
}

impl Default for NetworkServerCodec {
    fn default() -> NetworkServerCodec {
        NetworkServerCodec{prefix: false}
    }
}

impl Decoder for NetworkServerCodec
{
    type Item = Request;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.prefix {
            if src.len() < 11 {
                return Ok(None)
            }
            if &src[..11] == PREFIX {
                src.split_to(11);
                self.prefix = true;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Prefix mismatch"))
            }
        }
        
        let size = {
            if src.len() < 2 {
                return Ok(None)
            }
            NetworkEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<Request>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for NetworkServerCodec
{
    type Item = Response;
    type Error = io::Error;

    fn encode(&mut self, msg: Response, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match msg {
            Response::Handshake => dst.extend_from_slice(PREFIX),
            _ => {
                let msg = json::to_string(&msg).unwrap();
                let msg_ref: &[u8] = msg.as_ref();

                dst.reserve(msg_ref.len() + 2);
                dst.put_u16::<NetworkEndian>(msg_ref.len() as u16);
                dst.put(msg_ref);
            }
        }

        Ok(())
    }
}


/// Codec for Server -> Client transport
pub struct NetworkClientCodec {
    prefix: bool,
}

impl Default for NetworkClientCodec {
    fn default() -> NetworkClientCodec {
        NetworkClientCodec{prefix: false}
    }
}

impl Decoder for NetworkClientCodec
{
    type Item = Response;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.prefix {
            if src.len() < 11 {
                return Ok(None)
            }
            if &src[..11] == PREFIX {
                src.split_to(11);
                self.prefix = true;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Prefix mismatch"))
            }
        }

        let size = {
            if src.len() < 2 {
                return Ok(None)
            }
            NetworkEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(json::from_slice::<Response>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for NetworkClientCodec
{
    type Item = Request;
    type Error = io::Error;

    fn encode(&mut self, msg: Request, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if let Request::Handshake(_) = msg {
            dst.extend_from_slice(PREFIX);
        }

        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16::<NetworkEndian>(msg_ref.len() as u16);
        dst.put(msg_ref);
        Ok(())
    }
}
