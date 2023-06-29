use crate::lobby::Lobby;
use crate::web_socket::WebSocketConnection;
use actix::Addr;
use actix_web::{get, web::Data, web::Payload, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use uuid::Uuid;

#[get("/{group_id}")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    srv: Data<Addr<Lobby>>,
) -> Result<HttpResponse, Error> {
    let ws = WebSocketConnection::new(Uuid::new_v4(), srv.get_ref().clone());

    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}
