use std::io::{self, BufRead, Write};

/// I/O adapter for session participants.
/// Abstracts over stdin/stdout (humans) vs JSON pipes (AI).
pub trait SessionIO {
    /// Read one line of input. Returns None on EOF.
    fn read_line(&mut self) -> io::Result<Option<String>>;
    /// Write output to the participant.
    fn write_output(&mut self, msg: &str) -> io::Result<()>;
    /// Write a prompt string (e.g., "> ").
    fn write_prompt(&mut self, prompt: &str) -> io::Result<()>;
}

/// Stdin/stdout adapter for human CLI players.
pub struct StdIO {
    reader: io::BufReader<io::Stdin>,
}

impl StdIO {
    pub fn new() -> Self {
        StdIO {
            reader: io::BufReader::new(io::stdin()),
        }
    }
}

impl Default for StdIO {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionIO for StdIO {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            Ok(None) // EOF
        } else {
            Ok(Some(line.trim_end().to_string()))
        }
    }

    fn write_output(&mut self, msg: &str) -> io::Result<()> {
        println!("{}", msg);
        Ok(())
    }

    fn write_prompt(&mut self, prompt: &str) -> io::Result<()> {
        print!("{}", prompt);
        io::stdout().flush()
    }
}

/// JSON-line pipe adapter for AI participants.
/// Reads/writes one JSON object per line (newline-delimited JSON).
pub struct JsonPipeIO {
    reader: io::BufReader<io::Stdin>,
}

impl JsonPipeIO {
    pub fn new() -> Self {
        JsonPipeIO {
            reader: io::BufReader::new(io::stdin()),
        }
    }
}

impl Default for JsonPipeIO {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionIO for JsonPipeIO {
    fn read_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(line.trim_end().to_string()))
        }
    }

    fn write_output(&mut self, msg: &str) -> io::Result<()> {
        // Write JSON as a single line followed by newline
        println!("{}", msg);
        io::stdout().flush()
    }

    fn write_prompt(&mut self, _prompt: &str) -> io::Result<()> {
        // No prompt for AI participants — they read JSON lines
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Can't easily test stdin/stdout in unit tests, but we verify construction.
    #[test]
    fn json_pipe_construction() {
        // Just verify the type exists and can be constructed.
        // Actual I/O would need integration tests.
        let _: fn() -> JsonPipeIO = JsonPipeIO::new;
    }
}
