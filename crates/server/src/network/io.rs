use crate::network::ServerError;
use serde::{de::DeserializeOwned, Serialize};
use wtransport::{RecvStream, SendStream};

/// Read a length-prefixed bincode message from a WebTransport stream.
pub async fn read_message<T>(stream: &mut RecvStream) -> Result<T, ServerError>
where
    T: DeserializeOwned,
{
    let mut len_buf = [0u8; 4];
    read_exact(stream, &mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    read_exact(stream, &mut data).await?;

    Ok(bincode::deserialize(&data)?)
}

/// Serialize a message with a length prefix and send it over the stream.
pub async fn write_message<T>(stream: &mut SendStream, message: &T) -> Result<(), ServerError>
where
    T: Serialize,
{
    let data = bincode::serialize(message)?;
    let len = (data.len() as u32).to_be_bytes();

    stream
        .write_all(&len)
        .await
        .map_err(|err| ServerError::Transport(err.to_string()))?;
    stream
        .write_all(&data)
        .await
        .map_err(|err| ServerError::Transport(err.to_string()))?;

    Ok(())
}

async fn read_exact(stream: &mut RecvStream, buf: &mut [u8]) -> Result<(), ServerError> {
    let mut offset = 0;
    while offset < buf.len() {
        match stream
            .read(&mut buf[offset..])
            .await
            .map_err(|err| ServerError::Transport(err.to_string()))?
        {
            Some(0) => return Err(ServerError::ConnectionClosed),
            Some(n) => offset += n,
            None => return Err(ServerError::ConnectionClosed),
        }
    }
    Ok(())
}
