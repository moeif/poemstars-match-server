use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeEvent};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
mod common;
mod gamematch;
mod gameplay;
mod petable;
mod poemtable;
mod proto;
mod robot;
mod utils;

fn main() {
    let (tx, rx) = mpsc::channel();

    let (handler, listener) = node::split();
    let server_handler = handler.clone();
    thread::spawn(move || {
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
                            "Sync to client: {:?} - {:?} - {}",
                            endpoint_id1, endpoint_id2, json_str
                        );
                        let data = json_str.as_bytes();
                        if let Some(endpoint_id1) = endpoint_id1 {
                            if let Some(client_endpoint) = clients.get(&endpoint_id1) {
                                server_handler.network().send(*client_endpoint, data);
                            }
                        }

                        if let Some(endpoint_id2) = endpoint_id2 {
                            if let Some(client_endpoint) = clients.get(&endpoint_id2) {
                                server_handler.network().send(*client_endpoint, data);
                            }
                        }
                    }
                },
            })
        }
    });

    // 当前在游戏中的玩家，开始匹配的时间
    let mut gaming_player_map: HashMap<String, i64> = HashMap::new();
    let mut match_controller = gamematch::MatchController::new();
    let mut match_game_controller = gameplay::MatchGameController::new();

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

                                        let match_request = gamematch::MatchRequest {
                                            endpoint_id: Some(endpoint_id.clone()),
                                            player_id: match_info.id.clone(),
                                            player_name: match_info.name.clone(),
                                            player_level: match_info.level,
                                            player_elo_score: match_info.elo_score,
                                            player_correct_rate: match_info.correct_rate,
                                            timestamp: curr_timestamp,
                                        };

                                        gaming_player_map
                                            .insert(match_info.id, start_match_timestamp);
                                        match_controller.add_match(match_request);
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

        if let Some((some_match_request1, some_match_request2)) =
            match_controller.update_matches(curr_timestamp)
        {
            if let Some(match_request1) = some_match_request1 {
                let game_player1 =
                    gameplay::create_player_from_match(match_request1, curr_timestamp);
                let game_player2 = if let Some(match_request2) = some_match_request2 {
                    gameplay::create_player_from_match(match_request2, curr_timestamp)
                } else {
                    match_game_controller.create_robot_player(&game_player1, curr_timestamp)
                };

                if let Some(start_game_signal) =
                    match_game_controller.start_new_game(game_player1, game_player2)
                {
                    handler.signals().send(start_game_signal);
                }
            } else {
                println!("逻辑错误，匹配返回Some时第一个玩家不可能为None");
            }
        }

        // {
        //     // 将这两个玩家匹配到一起，创建一个游戏, 然后同步游戏开始消息
        //     if let Some(start_game_signal) =
        //         match_game_controller.create_new_game(match_result, curr_timestamp)
        //     {
        //         handler.signals().send(start_game_signal);
        //     }
        // }
    }
}
