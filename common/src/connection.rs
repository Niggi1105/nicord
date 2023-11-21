use crate::framing::*;
use anyhow::Result;
use std::fmt::Debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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
