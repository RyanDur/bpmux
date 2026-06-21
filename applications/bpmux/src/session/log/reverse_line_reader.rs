use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::LogError;

pub(super) const REVERSE_READ_CHUNK: u64 = 64 * 1024;

pub(super) struct ReverseLineReader {
    file: File,
    unread_until: u64,
    partial_head: Vec<u8>,
    ready: VecDeque<String>,
    drained: bool,
}

impl ReverseLineReader {
    pub(super) fn open(path: &Path) -> Result<Self, LogError> {
        let mut file = File::open(path)?;
        let unread_until = file.seek(SeekFrom::End(0))?;
        Ok(Self {
            file,
            unread_until,
            partial_head: Vec::new(),
            ready: VecDeque::new(),
            drained: false,
        })
    }

    pub(super) fn next_line(&mut self) -> Result<Option<String>, LogError> {
        while self.ready.is_empty() && !self.drained {
            self.pull_previous_chunk()?;
        }
        Ok(self.ready.pop_front())
    }

    fn pull_previous_chunk(&mut self) -> Result<(), LogError> {
        if self.unread_until == 0 {
            self.drained = true;
            let head = std::mem::take(&mut self.partial_head);
            self.enqueue_newest_first(head)?;
            return Ok(());
        }
        let size = REVERSE_READ_CHUNK.min(self.unread_until);
        self.unread_until -= size;
        self.file.seek(SeekFrom::Start(self.unread_until))?;
        let mut chunk = vec![0u8; size as usize];
        self.file.read_exact(&mut chunk)?;
        chunk.extend_from_slice(&self.partial_head);
        self.partial_head.clear();
        match chunk.iter().position(|&b| b == b'\n') {
            Some(first_newline) => {
                let complete = chunk.split_off(first_newline + 1);
                chunk.truncate(first_newline);
                self.partial_head = chunk;
                self.enqueue_newest_first(complete)?;
            }
            None => {
                self.partial_head = chunk;
            }
        }
        Ok(())
    }

    fn enqueue_newest_first(&mut self, bytes: Vec<u8>) -> Result<(), LogError> {
        let mut segments: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
        for segment in segments.drain(..).rev() {
            if segment.is_empty() {
                continue;
            }
            let line = String::from_utf8(segment.to_vec()).map_err(|e| {
                LogError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })?;
            self.ready.push_back(line);
        }
        Ok(())
    }
}
