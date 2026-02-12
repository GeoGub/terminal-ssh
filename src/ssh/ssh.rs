use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal,
};
use ssh2::Session;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::{thread, time::Duration};
use tokio::sync::{mpsc, oneshot};

pub async fn run_ssh_shell() -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect("192.168.64.154:22").unwrap();

    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password("admin", "12345678").unwrap();

    let mut channel = sess.channel_session().unwrap();
    channel.request_pty("xterm", None, None)?;
    channel.shell()?;

    sess.set_blocking(false);

    terminal::enable_raw_mode().unwrap();

    let (kbd_tx, mut kbd_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let (ssh_stdout_tx, mut ssh_stdout_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    let _ = tokio::task::spawn_blocking(move || {
        println!("stdin function");
        let kbd_tx = kbd_tx.clone();
        let stop_tx = std::sync::Mutex::new(Some(stop_tx));
        loop {
            match read() {
                Ok(Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                })) => match code {
                    KeyCode::Char(c) => {
                        if modifiers == KeyModifiers::CONTROL && c == 'c' {
                            let _ = kbd_tx.send(vec![0x03]);
                        } else if modifiers == KeyModifiers::CONTROL && c == 'd' {
                            let _ = kbd_tx.send(vec![0x04]);
                        } else {
                            let _ = kbd_tx.send(vec![c as u8]);
                        }
                    }
                    KeyCode::Enter => {
                        let _ = kbd_tx.send(vec![b'\n']);
                    }
                    KeyCode::Backspace => {
                        let _ = kbd_tx.send(vec![0x08]);
                    }
                    KeyCode::Tab => {
                        let _ = kbd_tx.send(vec![b'\t']);
                    }
                    KeyCode::Esc => {
                        if let Some(tx) = stop_tx.lock().unwrap().take() {
                            let _ = tx.send(());
                        }
                        break;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    });

    let _ = tokio::task::spawn_blocking(move || {
        println!("ssh read stdout function");
        let mut ssh_stdin = channel.stream(0);
        let mut buf = [0u8; 4096];

        loop {
            while let Ok(bytes) = kbd_rx.try_recv() {
                let _ = ssh_stdin.write_all(&bytes);
            }

            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = ssh_stdout_tx.send(buf[..n].to_vec());
                }
                _ => thread::sleep(Duration::from_millis(50)),
            }
            if channel.eof() {
                break;
            }
        }
    });

    let _ = tokio::spawn(async move {
        println!("print handle");
        while let Some(chunk) = ssh_stdout_rx.recv().await {
            print!("{}", String::from_utf8_lossy(&chunk));
            let _ = io::stdout().flush();
        }
    });

    let _ = stop_rx.await;
    terminal::disable_raw_mode().ok();

    Ok(())
}
