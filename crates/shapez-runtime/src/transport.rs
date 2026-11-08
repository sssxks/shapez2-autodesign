//! Native host execution and acceptance support.
use anyhow::{Context, Result, ensure};
use std::{
    fs::File,
    io::{Read, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

/// Maximum accepted native message payload in bytes (64 MiB).
const MAX_FRAME: usize = 64 * 1024 * 1024;

/// One worker owns all blocking pipe I/O. Killing the owned host closes its peer
/// and releases the worker even when a request times out during a write.
pub(crate) struct Connection {
    /// Queued serialized requests; closing the channel stops the worker.
    outgoing: Option<Sender<Vec<u8>>>,
    /// Response frames or worker errors.
    pub incoming: Receiver<std::result::Result<Vec<u8>, String>>,
    /// Blocking pipe worker thread.
    worker: Option<JoinHandle<()>>,
}

impl Connection {
    /// Start the pipe worker and receive readiness before processing requests.
    pub fn new(mut pipe: File) -> Self {
        let (outgoing, requests) = mpsc::channel::<Vec<u8>>();
        let (responses, incoming) = mpsc::channel();
        let worker = thread::spawn(move || {
            let result = (|| -> Result<()> {
                let ready = read_frame(&mut pipe)?;
                if responses.send(Ok(ready)).is_err() {
                    return Ok(());
                }
                for request in requests {
                    write_frame(&mut pipe, &request)?;
                    let response = read_frame(&mut pipe)?;
                    if responses.send(Ok(response)).is_err() {
                        break;
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                let _ = responses.send(Err(format!("{error:#}")));
            }
        });
        Self {
            outgoing: Some(outgoing),
            incoming,
            worker: Some(worker),
        }
    }

    /// Queue a serialized request for the blocking pipe worker.
    pub fn send(&self, value: Vec<u8>) -> Result<()> {
        self.outgoing.as_ref().context("Pipe closed")?.send(value)?;
        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.outgoing.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Read a nonempty bounded payload prefixed by a little-endian u32 length.
fn read_frame(reader: &mut impl Read) -> Result<Vec<u8>> {
    let mut header = [0; 4];
    reader
        .read_exact(&mut header)
        .context("Reading pipe header")?;
    let size = u32::from_le_bytes(header) as usize;
    ensure!(
        (1..=MAX_FRAME).contains(&size),
        "Invalid pipe frame size: {size}"
    );
    let mut bytes = vec![0; size];
    reader
        .read_exact(&mut bytes)
        .context("Reading pipe payload")?;
    Ok(bytes)
}

/// Write and flush a bounded payload with its little-endian u32 length prefix.
fn write_frame(writer: &mut impl Write, bytes: &[u8]) -> Result<()> {
    ensure!(
        (1..=MAX_FRAME).contains(&bytes.len()),
        "Pipe frame exceeds {MAX_FRAME} bytes"
    );
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn frames_survive_fragmented_reads_and_keep_message_boundaries() {
        struct Fragmented(Cursor<Vec<u8>>);
        impl Read for Fragmented {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let size = buf.len().min(3);
                self.0.read(&mut buf[..size])
            }
        }
        let first = "形状\n配方".as_bytes();
        let second = &[0_u8, 255, 3];
        let mut bytes = Vec::new();
        write_frame(&mut bytes, first).unwrap();
        write_frame(&mut bytes, second).unwrap();
        let mut reader = Fragmented(Cursor::new(bytes));
        assert_eq!(read_frame(&mut reader).unwrap(), first);
        assert_eq!(read_frame(&mut reader).unwrap(), second);
    }

    #[test]
    fn rejects_invalid_lengths_and_truncated_payloads() {
        for size in [0u32, MAX_FRAME as u32 + 1, u32::MAX] {
            assert!(read_frame(&mut Cursor::new(size.to_le_bytes())).is_err());
        }
        for bytes in [vec![3, 0, 0, 0, b'{'], vec![1, 0, 0]] {
            assert!(read_frame(&mut Cursor::new(bytes)).is_err());
        }
    }
}
