use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Handler,
    Running, StreamHandler, WrapFuture,
};
use actix_web_actors::ws;
use actix_web_actors::ws::WebsocketContext;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::messages::{ClientActorMessage, Connect, Disconnect, WsMessage};
use crate::Lobby;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

pub struct WebSocketConnection {
    pub id: Uuid,                // unique actor id
    pub room: Uuid,              // unique actor's room id
    pub lobby_addr: Addr<Lobby>, // actor's lobby address
    pub heart_beat: Instant,     // actor connection heartbeat
}

impl WebSocketConnection {
    pub fn new(room: Uuid, lobby: Addr<Lobby>) -> WebSocketConnection {
        WebSocketConnection {
            id: Uuid::new_v4(),
            room,
            lobby_addr: lobby,
            heart_beat: Instant::now(),
        }
    }
}

impl Actor for WebSocketConnection {
    // actor context
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();

        // initiate heart beat loop to keep connection alive
        self.heart_beat(ctx);

        // send connect message to actor
        self.lobby_addr
            .send(Connect {
                addr: addr.recipient(),
                lobby_id: self.room,
                self_id: self.id,
            })
            .into_actor(self)
            .then(
                // stop actor if message fails
                |res, _, ctx| {
                    match res {
                        Ok(_) => {}
                        Err(_) => ctx.stop(),
                    }
                    fut::ready(())
                },
            )
            .wait(ctx)
    }

    // disconnect from actor
    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        self.lobby_addr.do_send(Disconnect {
            id: self.id,
            room_id: self.room,
        });
        // stop actor anyways
        Running::Stop
    }
}

impl WebSocketConnection {
    fn heart_beat(&self, ctx: &mut WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
            if Instant::now().duration_since(actor.heart_beat) > CLIENT_TIMEOUT {
                println!("Disconnecting actor due to failed heartbeat");
                // TODO: send to trace
                // TODO: send admin alert email
                ctx.stop();
                return;
            }

            ctx.ping(b"hello from near event actor");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heart_beat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heart_beat = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => (),
            Ok(ws::Message::Text(s)) => self.lobby_addr.do_send(ClientActorMessage {
                id: self.id,
                msg: s.to_string(),
                room_id: self.room,
            }),
            Err(e) => std::panic::panic_any(e),
        }
    }
}

impl Handler<WsMessage> for WebSocketConnection {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}
