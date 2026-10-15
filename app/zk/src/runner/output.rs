//! Bounded diagnostic transport, deliberately outside Tokio's blocking pool.
use std::{fs::File, io, io::Write, os::fd::AsFd, time::Duration};
use tokio::sync::{mpsc, oneshot};

pub(super) struct Output {
    pub rows: mpsc::Sender<Vec<u8>>,
    done: oneshot::Receiver<io::Result<()>>,
}

impl Output {
    pub fn stdout() -> io::Result<Self> {
        // Do not retain std's global stdout lock: main's exit-time cleanup
        // must remain independent of this potentially blocked writer.
        let mut stdout = File::from(io::stdout().as_fd().try_clone_to_owned()?);
        let (rows, mut receiver) = mpsc::channel::<Vec<u8>>(16);
        let (finished, done) = oneshot::channel();
        // A blocked pipe write cannot safely be cancelled. This ordinary OS
        // thread owns only diagnostics, never native commands or their receipts.
        // Its handle is deliberately detached: process exit must not await it.
        std::thread::Builder::new()
            .name("longcat-stdout".into())
            .spawn(move || {
                let result = (|| {
                    while let Some(row) = receiver.blocking_recv() {
                        stdout.write_all(&row)?;
                    }
                    stdout.flush()
                })();
                let _ = finished.send(result);
            })?;
        Ok(Self { rows, done })
    }

    pub async fn finish(self) -> io::Result<()> {
        drop(self.rows);
        // This bounds total drain, not each row. Timeout abandons the detached
        // writer; it does not claim that a blocked write was cancelled/joined.
        tokio::time::timeout(Duration::from_secs(2), self.done)
            .await
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "diagnostic stdout drain stalled; writer detached",
                )
            })?
            .map_err(io::Error::other)?
    }
}
