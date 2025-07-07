/// Inter-process communication for multi-process architecture
/// 
/// Provides reliable communication channels between keeper, proxy, and inside processes
use crate::multiprocess::IpcMessage;
use crate::types::{IsolateError, Result};

use std::os::unix::io::RawFd;

/// IPC channel for communication between processes
pub struct IpcChannel {
    read_fd: Option<RawFd>,
    write_fd: Option<RawFd>,
}

impl IpcChannel {
    /// Create a new IPC channel using pipes
    pub fn new() -> Result<(Self, Self)> {
        let (read_fd, write_fd) = nix::unistd::pipe()
            .map_err(|e| IsolateError::Process(format!("Failed to create pipe: {}", e)))?;

        let reader = IpcChannel {
            read_fd: Some(read_fd),
            write_fd: None,
        };

        let writer = IpcChannel {
            read_fd: None,
            write_fd: Some(write_fd),
        };

        Ok((reader, writer))
    }

    /// Send a message through the channel
    pub fn send(&mut self, message: &IpcMessage) -> Result<()> {
        if let Some(fd) = self.write_fd {
            let serialized = serde_json::to_vec(message)
                .map_err(|e| IsolateError::Process(format!("Failed to serialize message: {}", e)))?;

            // Write message length first
            let len = serialized.len() as u32;
            let len_bytes = len.to_le_bytes();

            nix::unistd::write(fd, &len_bytes)
                .map_err(|e| IsolateError::Process(format!("Failed to write message length: {}", e)))?;

            // Write message data
            nix::unistd::write(fd, &serialized)
                .map_err(|e| IsolateError::Process(format!("Failed to write message: {}", e)))?;

            Ok(())
        } else {
            Err(IsolateError::Process("No write file descriptor".to_string()))
        }
    }

    /// Receive a message from the channel (non-blocking)
    pub fn try_recv(&mut self) -> Result<Option<IpcMessage>> {
        if let Some(fd) = self.read_fd {
            // Set non-blocking mode
            let flags = nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_GETFL)
                .map_err(|e| IsolateError::Process(format!("Failed to get flags: {}", e)))?;
            
            nix::fcntl::fcntl(fd, nix::fcntl::FcntlArg::F_SETFL(nix::fcntl::OFlag::from_bits_truncate(flags) | nix::fcntl::OFlag::O_NONBLOCK))
                .map_err(|e| IsolateError::Process(format!("Failed to set non-blocking: {}", e)))?;

            // Try to read message length
            let mut len_bytes = [0u8; 4];
            match nix::unistd::read(fd, &mut len_bytes) {
                Ok(4) => {
                    let len = u32::from_le_bytes(len_bytes) as usize;
                    
                    // Read message data
                    let mut buffer = vec![0u8; len];
                    match nix::unistd::read(fd, &mut buffer) {
                        Ok(bytes_read) if bytes_read == len => {
                            let message: IpcMessage = serde_json::from_slice(&buffer)
                                .map_err(|e| IsolateError::Process(format!("Failed to deserialize message: {}", e)))?;
                            Ok(Some(message))
                        }
                        Ok(_) => Err(IsolateError::Process("Incomplete message read".to_string())),
                        Err(nix::errno::Errno::EAGAIN) => Ok(None), // No data available
                        Err(e) => Err(IsolateError::Process(format!("Failed to read message data: {}", e))),
                    }
                }
                Ok(_) => Err(IsolateError::Process("Incomplete length read".to_string())),
                Err(nix::errno::Errno::EAGAIN) => Ok(None), // No data available
                Err(e) => Err(IsolateError::Process(format!("Failed to read message length: {}", e))),
            }
        } else {
            Err(IsolateError::Process("No read file descriptor".to_string()))
        }
    }

    /// Receive a message from the channel (blocking)
    pub fn recv(&mut self) -> Result<IpcMessage> {
        if let Some(fd) = self.read_fd {
            // Read message length
            let mut len_bytes = [0u8; 4];
            nix::unistd::read(fd, &mut len_bytes)
                .map_err(|e| IsolateError::Process(format!("Failed to read message length: {}", e)))?;

            let len = u32::from_le_bytes(len_bytes) as usize;
            
            // Read message data
            let mut buffer = vec![0u8; len];
            nix::unistd::read(fd, &mut buffer)
                .map_err(|e| IsolateError::Process(format!("Failed to read message data: {}", e)))?;

            let message: IpcMessage = serde_json::from_slice(&buffer)
                .map_err(|e| IsolateError::Process(format!("Failed to deserialize message: {}", e)))?;

            Ok(message)
        } else {
            Err(IsolateError::Process("No read file descriptor".to_string()))
        }
    }

    /// Close the channel
    pub fn close(&mut self) {
        if let Some(fd) = self.read_fd.take() {
            let _ = nix::unistd::close(fd);
        }
        if let Some(fd) = self.write_fd.take() {
            let _ = nix::unistd::close(fd);
        }
    }
}

impl Drop for IpcChannel {
    fn drop(&mut self) {
        self.close();
    }
}

/// Error pipe for critical error reporting
pub struct ErrorPipe {
    channel: IpcChannel,
}

impl ErrorPipe {
    /// Create a new error pipe
    pub fn new() -> Result<(Self, Self)> {
        let (reader, writer) = IpcChannel::new()?;
        Ok((
            ErrorPipe { channel: reader },
            ErrorPipe { channel: writer },
        ))
    }

    /// Send an error message
    pub fn send_error(&mut self, error: &str) -> Result<()> {
        let message = IpcMessage::ProcessError {
            error: error.to_string(),
        };
        self.channel.send(&message)
    }

    /// Try to receive an error message
    pub fn try_recv_error(&mut self) -> Result<Option<String>> {
        match self.channel.try_recv()? {
            Some(IpcMessage::ProcessError { error }) => Ok(Some(error)),
            Some(_) => Ok(None), // Ignore non-error messages
            None => Ok(None),
        }
    }
}

/// Status pipe for resource monitoring updates
pub struct StatusPipe {
    channel: IpcChannel,
}

impl StatusPipe {
    /// Create a new status pipe
    pub fn new() -> Result<(Self, Self)> {
        let (reader, writer) = IpcChannel::new()?;
        Ok((
            StatusPipe { channel: reader },
            StatusPipe { channel: writer },
        ))
    }

    /// Send a status update
    pub fn send_status(&mut self, message: IpcMessage) -> Result<()> {
        self.channel.send(&message)
    }

    /// Try to receive a status update
    pub fn try_recv_status(&mut self) -> Result<Option<IpcMessage>> {
        self.channel.try_recv()
    }

    /// Receive a status update (blocking)
    pub fn recv_status(&mut self) -> Result<IpcMessage> {
        self.channel.recv()
    }
}