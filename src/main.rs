use ssh2::Session;
use std::io::{Read, Write, stdout};
use std::net::TcpStream;
use std::{thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tcp = TcpStream::connect("ip:22").unwrap();
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    sess.userauth_password("username", "password").unwrap();
    let mut channel = sess.channel_session().unwrap();
    let mut ssh_stdin = channel.stream(0);
    channel.request_pty("xterm", None, None).unwrap();
    channel.shell()?;

    sess.set_blocking(false);

    let stdin_thread = thread::spawn(move || {
        let mut input = String::new();
        loop {
            input.clear();
            if let Ok(_) = std::io::stdin().read_line(&mut input) {
                if input.trim() == "exit" {
                    break;
                }
                ssh_stdin.write(input.as_bytes()).unwrap();
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
                Ok(0) => break,
                _ => thread::sleep(Duration::from_millis(200)),
            }
        }
    });

    [ stdout_thread, stdin_thread ].into_iter().for_each(|t| { let _ = t.join(); } );
    Ok(())
}
