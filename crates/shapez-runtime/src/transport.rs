use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

const MAX_FRAME: usize = 64 * 1024 * 1024;

/// One worker owns all blocking pipe I/O. Killing the owned host closes its peer
/// and releases the worker even when a request times out during a write.
pub(crate) struct Connection {
    outgoing: Option<Sender<Value>>,
    pub incoming: Receiver<std::result::Result<Value, String>>,
    worker: Option<JoinHandle<()>>,
}

impl Connection {
    pub fn new(mut pipe: File) -> Self {
        let (outgoing, requests) = mpsc::channel();
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

    pub fn send(&self, value: Value) -> Result<()> {
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

fn read_frame(reader: &mut impl Read) -> Result<Value> {
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
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_frame(writer: &mut impl Write, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(
        bytes.len() <= MAX_FRAME,
        "Pipe frame exceeds {MAX_FRAME} bytes"
    );
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&bytes)?;
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
        let first = serde_json::json!({"id": 1, "text": "形状\n配方"});
        let second = serde_json::json!({"id": 2, "ok": false});
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &first).unwrap();
        write_frame(&mut bytes, &second).unwrap();
        let mut reader = Fragmented(Cursor::new(bytes));
        assert_eq!(read_frame(&mut reader).unwrap(), first);
        assert_eq!(read_frame(&mut reader).unwrap(), second);
    }

    #[test]
    fn rejects_invalid_lengths_truncated_payloads_and_invalid_json() {
        for size in [0u32, MAX_FRAME as u32 + 1, u32::MAX] {
            assert!(read_frame(&mut Cursor::new(size.to_le_bytes())).is_err());
        }
        for bytes in [vec![3, 0, 0, 0, b'{'], vec![1, 0, 0, 0, b'!']] {
            assert!(read_frame(&mut Cursor::new(bytes)).is_err());
        }
    }
}
