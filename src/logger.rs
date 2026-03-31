use colored::*;

#[derive(Clone, Copy)]
pub enum LogLevel {
    Info,
    Debug,
    Asm,
}

pub struct Logger {
    pub level: LogLevel,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Logger { level }
    }

    pub fn info(&self, msg: &str) {
        if matches!(self.level, LogLevel::Info | LogLevel::Debug)
            && !matches!(self.level, LogLevel::Asm)
        {
            println!("   {}", msg.bold());
        }
    }

    pub fn debug(&self, msg: &str) {
        if matches!(self.level, LogLevel::Debug) && !matches!(self.level, LogLevel::Asm) {
            println!("     [DEBUG] {}", msg.dimmed());
        }
    }
}

#[macro_export]
macro_rules! emit {
    ($($arg:tt)*) => {
        println!("    {}", format!($($arg)*))
    }
}

#[macro_export]
macro_rules! label {
    ($($arg:tt)*) => {
        println!("{}", format!($($arg)*))
    }
}
