use ansi_term::{Color, Style};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref NO_COLOR: bool = std::env::var("NO_COLOR").is_ok();
    pub static ref BLUE: Style = Style::new().bold().fg(Color::Blue);
    pub static ref GREEN: Style = Style::new().bold().fg(Color::Green);
    pub static ref RED: Style = Style::new().bold().fg(Color::Red);
    pub static ref WHITE: Style = Style::new().bold().fg(Color::White);
}

#[macro_export]
macro_rules! color_println {
    ($style:tt, $($arg:tt)*) => {
        if *NO_COLOR {
            println!("{}", format!($($arg)*))
        } else {
            println!("{}", $style.paint(format!($($arg)*)))
        }
    }
}

#[macro_export]
macro_rules! color_eprintln {
    ($($arg:tt)*) => {
        if *NO_COLOR {
            eprintln!("{}", format!($($arg)*))
        } else {
            eprintln!("{}", RED.paint(format!($($arg)*)))
        }
    }
}
