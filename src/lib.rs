

use tokio::{io::AsyncWriteExt, net::TcpListener, net::TcpStream, sync::mpsc::Sender};
use std::{sync::Arc, vec};
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, WebSocketStream}; 
use tungstenite::protocol::Message; 
use futures_util::{stream::StreamExt, stream::SplitSink, SinkExt}; 
use tungstenite::handshake::server::{Request, Response};

pub fn start() -> WebPlotter {
    let wp = WebPlotter::new();
    {
        let wp_clone = wp.clone();
        tokio::spawn(async move {
            println!("Starting async");
            start_async(wp_clone).await;
            println!("Started async");
        });
    }
    wp
}
pub struct WebPlotter{
    plotter: Arc<WebPlotter_>
}
impl WebPlotter{
    pub fn new() -> Self {
        WebPlotter{
            plotter: Arc::new(WebPlotter_{
                sockets: Mutex::new(vec![]),
                messages: Mutex::new(vec![])
            })
        }
    }
    pub fn send(&self, str: String){
        {
            let plotter = self.plotter.clone();
            let s = str.clone();
            tokio::spawn(async move {
                plotter.send(s).await;
            });
        }
    }
    async fn handle_websocket(&self, stream: tokio::net::TcpStream){
        let plotter = self.plotter.clone();
        plotter.handle_websocket(stream).await;
    }
    pub fn clone(&self) -> Self {
        WebPlotter{
            plotter: self.plotter.clone()
        }
    }
}
struct WebPlotter_{
    sockets: Mutex<Vec<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    messages: Mutex<Vec<String>>
}

impl WebPlotter_{
    async fn handle_websocket(&self, stream: tokio::net::TcpStream){
        let ws_stream = match accept_async(stream).await{
            Ok(ws_stream) => ws_stream,
            Err(e) => {
                println!("Error during WebSocket handshake: {}", e);
                return;
            }
        };
        let (tx, _rx) = ws_stream.split();
        let mut sockets = self.sockets.lock().await;
        sockets.push(tx);
        println!("Added socket");
    }
    async fn send(&self, str: String){
        let mut messages = self.messages.lock().await;
        messages.push(str);
        let mut sockets = self.sockets.lock().await;
        let m = messages.pop().unwrap();
        for socket in sockets.iter_mut() {
            let message = Message::text(&m);
            socket.send(message).await.unwrap();
        }
    }
}
async fn start_async(wp: WebPlotter){
    let listener = TcpListener::bind("0.0.0.0:3999".to_string()).await.unwrap();
    loop {
        let (mut tcp_stream, _) = listener.accept().await.unwrap();
        println!("Accepted connection");
        let mut buf = vec![0; 1024];
        let n = tcp_stream.peek(&mut buf).await.unwrap();
        println!("Peeked {} bytes", n);
        if n == 0 {
            continue;
        }
        let request = String::from_utf8_lossy(&buf[..n]);

        if request.contains("Upgrade: websocket") {
            let wp = wp.clone();
            tokio::spawn(async move {
                println!("Handling WebSocket");
                wp.handle_websocket(tcp_stream).await;
            });
        } else {
            let response = response();
            tcp_stream.write_all(response.as_bytes()).await.unwrap();
            tcp_stream.shutdown().await.unwrap();
        }
    }

}

fn response() -> String {
    let body = r#"
    <html>
        <body>
            <h1>Hello, World!</h1>
            <script>
                (function() {
                    var ws = new WebSocket('ws://' + window.location.host + '/ws');
                    ws.onopen = function() {
                        console.log('WebSocket connection established');
                    };
                    ws.onmessage = function(event) {
                        var message = JSON.parse(event.data);
                        var div = document.createElement('div');
                        var title = document.createElement('h2');
                        title.textContent = message.title;
                        var img = document.createElement('img');
                        img.src = 'data:image/png;base64,' + message.bitmap;
                        div.appendChild(title);
                        div.appendChild(img);
                        document.body.appendChild(div);
                    };
                    ws.onerror = function(error) {
                        console.error('WebSocket error:', error);
                    };
                    ws.onclose = function() {
                        console.log('WebSocket connection closed');
                    };
                })();
            </script>
        </body>
    </html>
    "#;
    format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: {}\r\n\r\n{}", body.len(), body)
}