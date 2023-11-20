use std::fmt::Debug;

use anyhow::Result;
use log::{error, info};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::FramingError;

///a simple frame
///[LEN, T as json with length = LEN]
///provides generic enframing and defraiming methods
///supports up to 10mb large frames
pub trait Frameable<T = Self>
where
    Self: Serialize + DeserializeOwned,
{
    fn deframe(bytes: &[u8]) -> Result<Option<Self>> {
        if bytes.len() <= 7 {
            return Ok(None);
        }
        let bstr = String::from_utf8(bytes.to_vec())?;
        let l = bstr[0..7].parse::<usize>()?;
        if bstr.len() < l + 7 {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice::<Self>(&bytes[7..l + 7])?))
    }
    fn enframe(&self) -> Result<Vec<u8>>
    where
        Self: Serialize,
    {
        let str = serde_json::to_string(self)?;
        if str.len() + 7 > 9_999_999 {
            return Err(FramingError::MaximumFrameSizeExceeded.into());
        }
        let mut r = String::new();
        let l = (str.len() as u32).to_string();
        if l.len() < 7 {
            let t = String::from_utf8(vec![b'0'; 7 - l.len()])?;
            r.push_str(&t);
        }
        r.push_str(&l);
        r.push_str(&str);
        Ok(r.into_bytes())
    }
}

///a connection wraper for tokio tcp streams that supports framed writing and reading
#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
    pub async fn write<'a, T>(&mut self, data: T) -> Result<()>
    where
        T: Frameable,
    {
        let frame = data.enframe()?;
        let frame_len = frame.len();
        let mut n = 0;
        while n != frame_len {
            self.stream.writable().await?;
            n += self.stream.write(&frame[n..]).await?;
        }
        Ok(())
    }
    pub async fn read<T>(&mut self) -> Result<T>
    where
        T: Frameable + Debug,
    {
        let mut buffer = Vec::new();
        let v = loop {
            let _ = self.stream.read_buf(&mut buffer).await?;
            if let Some(val) = T::deframe(&buffer)? {
                info!("got something: {:?}", val);
                break val;
            };
        };
        Ok(v)
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::messages;

    #[tokio::test]
    async fn test_connection() {
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.bind("127.0.0.1:8087".parse().unwrap()).unwrap();
        let listener = socket.listen(32).unwrap();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut conn = Connection::new(stream);
            let msg = conn.read::<messages::Request>().await.unwrap();
            match msg {
                messages::Request::Ping(msg) => {
                    assert_eq!(msg, "hello world".to_string());
                }
                _other => {
                    assert!(false);
                }
            };
        });

        let writer = tokio::net::TcpSocket::new_v4().unwrap();
        let stream = writer
            .connect("127.0.0.1:8087".parse().unwrap())
            .await
            .unwrap();
        let mut conn = Connection::new(stream);
        conn.write(messages::Request::Ping("hello world".to_string()))
            .await
            .unwrap();
    }
}
