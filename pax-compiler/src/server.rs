/// Actix-based websocket Compiler Server —
/// During compilation, static binary (pure rust, no .pax yet) is built along with designtime dependencies
/// Binary is executed, and designtime connects to this websocket server to receive parsed .pax tokens.
/// This indirection through the compiler server allows for several advantages:
///   - Fundamentally, parsing .pax source code is no different than receiving data from a server.  This constraint enforces Pax's "designability"
///   -


use std::time::{Duration, Instant};

use actix::*;
use actix_files as fs;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, rt, middleware};
use actix_web_actors::ws;

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

use std::sync::{atomic::{AtomicUsize, Ordering}, Arc, mpsc};

use std::collections::{HashMap, HashSet};
use std::{thread, time};
use actix_web::dev::Server;


/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our websocket route
async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<CompilerServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsChatSession {
            id: 0,
            hb: Instant::now(),
            name: None,
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

///  Displays and affects state
async fn get_count(count: web::Data<Arc<AtomicUsize>>) -> impl Responder {
    let current_count = count.fetch_add(1, Ordering::SeqCst);
    format!("Visitors: {}", current_count)
}

struct WsChatSession {
    /// unique session id
    id: usize,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    /// peer name
    name: Option<String>,
    /// Chat server
    addr: Addr<CompilerServer>,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with CompilerServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<Message> for WsChatSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        println!("WEBSOCKET MESSAGE: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                // we check for /sss type of messages
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(2, ' ').collect();
                    match v[0] {
                        "/name" => {
                            if v.len() == 2 {
                                self.name = Some(v[1].to_owned());
                            } else {
                                ctx.text("!!! name is required");
                            }
                        }
                        _ => ctx.text(format!("!!! unknown command: {:?}", m)),
                    }
                } else {
                    let msg = if let Some(ref name) = self.name {
                        format!("{}: {}", name, m)
                    } else {
                        m.to_owned()
                    };
                    // send message to chat server
                    self.addr.do_send(ClientMessage {
                        id: self.id,
                        msg,
                    })
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl WsChatSession {
    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(Disconnect { id: act.id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}




pub fn start_ws_server() {
    std::env::set_var("RUST_LOG", "actix_web=info,actix_server=trace");
    // env_logger::init();

    let (tx, rx) = mpsc::channel();

    println!("START SERVER");
    thread::spawn(move || {
        let _ = start_ws_threaded(tx);
    });

    let srv = rx.recv().unwrap();

    println!("WAITING 10 SECONDS");
    thread::sleep(time::Duration::from_secs(30));

    println!("STOPPING SERVER");
    // init stop server and wait until server gracefully exit
    rt::System::new("").block_on(srv.stop(true));
}

fn start_ws_threaded(tx: mpsc::Sender<Server>) -> std::io::Result<()> {
    let mut sys = rt::System::new("test");

    // App state
    // We are keeping a count of the number of visitors
    let app_state = Arc::new(AtomicUsize::new(0));

    // Start chat server actor
    let server = CompilerServer::new(app_state.clone()).start();

    // Create Http server with websocket support
    let srv = HttpServer::new(move || {
        App::new()
            .data(app_state.clone())
            .data(server.clone())
            // redirect to websocket.html
            .service(web::resource("/").route(web::get().to(|| {
                HttpResponse::Found()
                    .header("LOCATION", "/static/websocket.html")
                    .finish()
            })))
            .route("/count/", web::get().to(get_count))
            // websocket
            .service(web::resource("/ws/").to(chat_route))
            // static resources
            .service(fs::Files::new("/static/", "static/"))
    })
        .bind("127.0.0.1:8080")?
        .run();

    // send server controller to main thread
    let _ = tx.send(srv.clone());

    // run future
    sys.block_on(srv)
}








/// `CompilerServer` is an actor. It maintains list of connection client session.
/// And manages available rooms. Peers send messages to other peers in same
/// room through `CompilerServer`.



/// Compiler server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// Message for compiler server communications

/// New compiler session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Data
    pub msg: String,
}



/// `CompilerServer` manages compiler rooms and responsible for coordinating compiler
/// session. implementation is super primitive
pub struct CompilerServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rng: ThreadRng,
    connection_count: Arc<AtomicUsize>,
}

impl CompilerServer {
    pub fn new(connection_count: Arc<AtomicUsize>) -> CompilerServer {
        CompilerServer {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
            connection_count: connection_count,
        }
    }
}

impl CompilerServer {
    fn send_message(&self, message: &str) {
        for id in &self.sessions {
            if let Some(addr) = self.sessions.get(&id.0) {
                let _ = addr.do_send(Message(message.to_owned()));
            }
        }
    }
}

/// Make actor from `CompilerServer`
impl Actor for CompilerServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for CompilerServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Client connected");

        // notify all users in same room
        self.send_message( "Someone joined");

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);


        let count = self.connection_count.fetch_add(1, Ordering::SeqCst);
        self.send_message(&format!("Total visitors {}", count));

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for CompilerServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        // remove address
        self.sessions.remove(&msg.id);

    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for CompilerServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(msg.msg.as_str());
    }
}

