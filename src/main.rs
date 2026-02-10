use ssh2::Session;
use std::io::{Read, Write, stdout};
use std::net::TcpStream;
use std::{thread, time::Duration};
use std::process::exit;

use crossterm::{terminal, event::{KeyEvent, KeyCode, Event, KeyEventKind, KeyModifiers, read}};

// #[tokio::main]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect("192.168.0.108:22").unwrap();
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password("admin", "12345678").unwrap();
    let mut channel = sess.channel_session().unwrap();
    let mut ssh_stdin = channel.stream(0);
    channel.request_pty("xterm", None, None)?;
    channel.shell()?;

    sess.set_blocking(false);
    terminal::enable_raw_mode()?;

    let stdin_thread = thread::spawn(move || {
        loop {
            match read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) => {
                    if modifiers == KeyModifiers::CONTROL && c.to_string() == "c" {
                        ssh_stdin.write(&[0x03]).unwrap();
                        continue;
                    } else if modifiers == KeyModifiers::CONTROL && c.to_string() == "d" {
                        ssh_stdin.write(&[0x04]).unwrap();
                        continue;
                    }
                    ssh_stdin.write(&[c as u8]).unwrap();
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    ssh_stdin.write(&[b'\n']).unwrap();
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    ssh_stdin.write(&[0x08]).unwrap();
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    terminal::disable_raw_mode().unwrap();
                    println!();
                    exit(0);
                },
                Event::Key(KeyEvent {
                    code: KeyCode::Tab,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    ssh_stdin.write(&[b'\t']).unwrap();
                },
                _ => println!("Unsupported key event: {:?}", read().unwrap()),
            }
        }
    });

    let stdout_thread = thread::spawn(move || {
        let mut buf = [0u8; 1024];
        loop {
            match channel.read(&mut buf) {
                Ok(c) if c > 0 => {
                    print!("{}", String::from_utf8_lossy(&buf[0..c]));
                    let _ = stdout().flush();
                }
                Ok(0) => {
                    terminal::disable_raw_mode().unwrap();
                    println!();
                    exit(0);
                },
                _ => thread::sleep(Duration::from_millis(10)),
            }
        }
    });

    [ stdout_thread, stdin_thread ].into_iter().for_each(|t| { let _ = t.join(); } );
    Ok(())
}
