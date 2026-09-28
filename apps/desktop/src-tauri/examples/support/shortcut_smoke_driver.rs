use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

pub struct Driver {
    child: Child,
    input: Option<ChildStdin>,
    lines: Receiver<String>,
}
impl Driver {
    pub fn start(path: &str, preset: &str) -> Result<Self, String> {
        let mut child = Command::new(path)
            .arg(std::process::id().to_string())
            .arg(preset)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().ok_or("Driver stdin missing")?;
        let output = child.stdout.take().ok_or("Driver stdout missing")?;
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                match line {
                    Ok(line) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let result = Self {
            child,
            input: Some(input),
            lines,
        };
        result.expect("ready")?;
        Ok(result)
    }
    fn expect(&self, expected: &str) -> Result<(), String> {
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| format!("Driver did not acknowledge {expected}"))?;
        if line != expected {
            return Err(format!("Driver returned {line:?}, expected {expected}"));
        }
        Ok(())
    }
    pub fn command(&mut self, command: &str) -> Result<(), String> {
        let input = self.input.as_mut().ok_or("Driver stdin closed")?;
        writeln!(input, "{command}").map_err(|e| e.to_string())?;
        input.flush().map_err(|e| e.to_string())?;
        self.expect(command)
    }
}
impl Drop for Driver {
    fn drop(&mut self) {
        // Even a failed assertion asks the live driver to balance all keys.
        if let Some(mut input) = self.input.take() {
            let _ = writeln!(input, "release\nquit");
            let _ = input.flush();
            // Close stdin before waiting. EOF also runs Swift's keyup defer.
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Never SIGKILL a helper that might still own keydowns: that bypasses
        // its cleanup. Closed stdin makes it exit after balancing its keys.
    }
}
