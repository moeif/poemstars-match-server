use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeEvent};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
mod common;
mod models;
mod petable;
mod proto;
mod utils;
use petable::PETable;
mod robot;

fn main() {
    let pe_table = PETable::new();

    let (tx, rx) = mpsc::channel();

    let (handler, listener) = node::split();
    let server_handler = handler.clone();
    let server_task = thread::spawn(move || {
        if let Ok((_, _)) = server_handler
            .network()
            .listen(Transport::Ws, "0.0.0.0:3044")
        {
            println!("Server Started!");
            let mut clients = HashMap::new();
            listener.for_each(move |event| match event {
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(_, _) => unreachable!(),
                    NetEvent::Accepted(_endpoint, _listener) => {
                        println!("Client connected: {:?}", _endpoint.resource_id());
                        let endpoint_id = _endpoint.resource_id().to_string();
                        clients.insert(endpoint_id, _endpoint);
                    }
                    NetEvent::Message(endpoint, data) => {
                        println!(
                            "Server Received: {:?}, data: {:?}",
                            endpoint.resource_id(),
                            String::from_utf8_lossy(data)
                        );
                        // handler.network().send(endpoint, data);
                        if let Ok(json_str) = std::str::from_utf8(data) {
                            let endpoint_id = endpoint.resource_id().to_string();
                            if let Ok(()) = tx.send((endpoint_id, json_str.to_string())) {
                            } else {
                                println!("channel send error!");
                            }
                        }
                    }
                    NetEvent::Disconnected(_endpoint) => {
                        let endpoint_id = _endpoint.resource_id().to_string();
                        println!("Client disconnected: {:?}", endpoint_id);
                        clients.remove(&endpoint_id);
                    }
                },
                NodeEvent::Signal(signal) => match signal {
                    common::Signal::Send(endpoint_id, json_str) => {
                        println!("Send Msg to client: {} - {}", endpoint_id, json_str);
                        if let Some(client_endpoint) = clients.get(&endpoint_id) {
                            let data = json_str.as_bytes();
                            server_handler.network().send(*client_endpoint, data);
                        }
                    }
                    common::Signal::Sync(endpoint_id1, endpoint_id2, json_str) => {
                        println!(
                            "Sync to client: {} - {} - {}",
                            endpoint_id1, endpoint_id2, json_str
                        );
                        let data = json_str.as_bytes();
                        if let Some(client_endpoint) = clients.get(&endpoint_id1) {
                            server_handler.network().send(*client_endpoint, data);
                        }

                        if let Some(client_endpoint) = clients.get(&endpoint_id2) {
                            server_handler.network().send(*client_endpoint, data);
                        }
                    }
                },
            })
        }
    });

    // 当前在游戏中的玩家，开始匹配的时间
    let mut gaming_player_map: HashMap<String, i64> = HashMap::new();
    let mut match_controller = models::MatchController::new();
    let mut match_game_controller = models::MatchGameController::new();

    // game server logic loop
    loop {
        let curr_timestamp = utils::get_timestamp();
        if let Ok((endpoint_id, json_str)) = rx.try_recv() {
            if let Ok(json_values) = serde_json::from_str::<Value>(&json_str) {
                if let Some(proto_id) = json_values["proto_id"].as_u64() {
                    if let Some(proto_json_str) = json_values["proto_json_str"].as_str() {
                        match proto_id {
                            proto::PROTO_CGSTARTMATCH => {
                                if let Ok(match_info) =
                                    serde_json::from_str::<proto::CGStartMatch>(proto_json_str)
                                {
                                    if !gaming_player_map.contains_key(&match_info.id) {
                                        let start_match_timestamp = curr_timestamp;
                                        let player_id = match_info.id.clone();
                                        let socket_endpoint_id = endpoint_id.clone();
                                        let matching_req = models::MatchingReq {
                                            endpoint_id: socket_endpoint_id,
                                            cg_match_info: match_info,
                                            start_match_timestamp,
                                        };

                                        gaming_player_map.insert(player_id, start_match_timestamp);
                                        match_controller.add_match(matching_req);
                                        // 回复消息，匹配中 CGStartMatch
                                        if let Some(proto_json_str) =
                                            proto::GCStartMatch::gc_to_json(0)
                                        {
                                            handler.signals().send(common::Signal::Send(
                                                endpoint_id,
                                                proto_json_str,
                                            ));
                                        }
                                    } else {
                                        // 玩家当前已经在匹配或游戏中，暂时不让进了，直接回复匹配失败
                                        if let Some(proto_json_str) =
                                            proto::GCStartMatch::gc_to_json(-1)
                                        {
                                            handler.signals().send(common::Signal::Send(
                                                endpoint_id,
                                                proto_json_str,
                                            ));
                                        }
                                    }
                                }
                            }
                            proto::PROTO_CGMATCHGAMEOPT => {
                                if let Ok(opt_info) =
                                    serde_json::from_str::<proto::CGMatchGameOpt>(proto_json_str)
                                {
                                    // if let Some(game) = match_game_map.get_mut(&opt_info.game_id) {
                                    //     // 设置玩家操作，会设置 dirty 状态，统一同步
                                    //     game.on_opt(opt_info);
                                    //     // 操作完后同步给两个玩家
                                    // }
                                    match_game_controller.on_opt(opt_info, curr_timestamp);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        if let Some(sync_signal_vec) = match_game_controller.update_games(curr_timestamp) {
            // 同步游戏
            for signal in sync_signal_vec {
                handler.signals().send(signal);
            }
        }
        if let Some(match_result) = match_controller.update_matches(curr_timestamp) {
            // 将这两个玩家匹配到一起，创建一个游戏, 然后同步游戏开始消息
            if let Some(start_game_signal) = match_game_controller.create_new_game(match_result) {
                handler.signals().send(start_game_signal);
            }
        }
    }
}
