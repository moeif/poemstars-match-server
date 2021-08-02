use chrono::prelude::*;
use message_io::network::{NetEvent, Transport};
use message_io::node::{self, NodeEvent};
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc;
use std::thread;
mod proto;

const MATCH_POEM_NUM: i32 = 10;
const MATCH_TIME: u8 = 5;

enum Signal {
    Send(String, String),
    Sync(String, String, String),
}

struct MatchingReq {
    pub endpoint_id: String,
    pub cg_match_info: proto::CGStartMatch,
    pub start_match_timestamp: i64,
}

#[derive(Serialize)]
struct MatchPlayer {
    pub id: String,
    pub endpoint_id: String,
    pub name: String,
    pub last_opt_timestamp: i64, // 最后一次的操作时间
    pub last_opt_index: i32,     // 最后一次操作的索引
    pub opt_bitmap: u32,         // 操作位数据, 0 正确，1 错误
    pub is_dirty: bool,
}

impl MatchPlayer {
    fn new(endpoint_id: String, id: String, name: String) -> Self {
        Self {
            id,
            endpoint_id,
            name,
            last_opt_timestamp: get_timestamp(),
            last_opt_index: -1,
            opt_bitmap: 0,
            is_dirty: false,
        }
    }

    fn is_all_opt_end(&self) -> bool {
        self.last_opt_index + 1 == MATCH_POEM_NUM
    }

    fn on_opt(&mut self, opt: proto::CGMatchGameOpt) {
        self.is_dirty = true;
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64) {
        self.is_dirty = true;
    }

    fn is_dirty(&mut self) -> bool {
        let tmp_is_dirty = self.is_dirty;
        self.is_dirty = false;
        return tmp_is_dirty;
    }
}

#[derive(Serialize)]
struct MatchGame {
    pub id: String,           // 游戏ID
    pub start_timestamp: i64, // 游戏开始时间戳
    pub player1: MatchPlayer,
    pub player2: MatchPlayer,
    pub is_gaming: bool, // 游戏进行中
    pub is_dirty: bool,
}

impl MatchGame {
    fn new(player1: MatchPlayer, player2: MatchPlayer) -> Self {
        let start_timestamp = get_timestamp();
        Self {
            id: format!("{}_{}_{}", player1.id, player2.id, start_timestamp),
            start_timestamp,
            player1,
            player2,
            is_gaming: true,
            is_dirty: false,
        }
    }

    fn is_dirty(&mut self) -> bool {
        let is_dirty = self.is_dirty || self.player1.is_dirty() || self.player2.is_dirty();
        self.is_dirty = false;
        return is_dirty;
    }

    fn on_opt(&mut self, opt: proto::CGMatchGameOpt) {
        if opt.id == self.player1.id {
            self.player1.on_opt(opt);
        } else if opt.id == self.player2.id {
            self.player2.on_opt(opt);
        }
    }

    // 更新游戏是否结束
    fn update_end_status(&mut self) {
        if self.player1.is_all_opt_end() && self.player2.is_all_opt_end() {
            self.is_gaming = false;
            self.is_dirty = true;
        }
    }

    fn is_game_end(&self) -> bool {
        !self.is_gaming
    }

    fn update_opt_timeout_status(&mut self, curr_timestamp: i64) {
        self.player1.update_opt_timeout_status(curr_timestamp);
        self.player2.update_opt_timeout_status(curr_timestamp);
    }

    fn gc_to_json(&self) -> Option<String> {
        if let Ok(json_str) = serde_json::to_string(self) {
            if let Ok(proto_data_json_str) =
                serde_json::to_string(&proto::ProtoData::new(proto::PROTO_UPDATEGAME, json_str))
            {
                return Some(proto_data_json_str);
            }
        }
        return None;
    }
}

fn main() {
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
                    Signal::Send(endpoint_id, json_str) => {
                        println!("Send Msg to client: {} - {}", endpoint_id, json_str);
                        if let Some(client_endpoint) = clients.get(&endpoint_id) {
                            let data = json_str.as_bytes();
                            server_handler.network().send(*client_endpoint, data);
                        }
                    }
                    Signal::Sync(endpoint_id1, endpoint_id2, json_str) => {
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

    // 保存所有的游戏
    // let mut games = HashMap::new();
    let mut matching_map: HashMap<u32, VecDeque<MatchingReq>> = HashMap::new();
    // 当前在游戏中的玩家，开始匹配的时间
    let mut gaming_player_map: HashMap<String, i64> = HashMap::new();
    // 保存当前正在进行中的游戏
    let mut match_game_map: HashMap<String, MatchGame> = HashMap::new();

    // game server logic loop
    loop {
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
                                        let start_match_timestamp = get_timestamp();
                                        let player_id = match_info.id.clone();
                                        let socket_endpoint_id = endpoint_id.clone();
                                        let level = match_info.level;
                                        let matching_req = MatchingReq {
                                            endpoint_id: socket_endpoint_id,
                                            cg_match_info: match_info,
                                            start_match_timestamp,
                                        };

                                        if !matching_map.contains_key(&level) {
                                            matching_map.insert(level, VecDeque::new());
                                        }

                                        if let Some(queue) = matching_map.get_mut(&level) {
                                            queue.push_back(matching_req);
                                            gaming_player_map
                                                .insert(player_id, start_match_timestamp);
                                            // 回复消息，匹配中 CGStartMatch
                                            if let Some(proto_json_str) =
                                                proto::GCStartMatch::gc_to_json(0)
                                            {
                                                handler.signals().send(Signal::Send(
                                                    endpoint_id,
                                                    proto_json_str,
                                                ));
                                            }
                                        }
                                    } else {
                                        // 玩家当前已经在匹配或游戏中，暂时不让进了，直接回复匹配失败
                                        if let Some(proto_json_str) =
                                            proto::GCStartMatch::gc_to_json(-1)
                                        {
                                            handler
                                                .signals()
                                                .send(Signal::Send(endpoint_id, proto_json_str));
                                        }
                                    }
                                }
                            }
                            proto::PROTO_CGMATCHGAMEOPT => {
                                if let Ok(opt_info) =
                                    serde_json::from_str::<proto::CGMatchGameOpt>(proto_json_str)
                                {
                                    if let Some(game) = match_game_map.get_mut(&opt_info.game_id) {
                                        // 设置玩家操作，会设置 dirty 状态，统一同步
                                        game.on_opt(opt_info);
                                        // 操作完后同步给两个玩家
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut ended_game = Vec::new(); // 保存哪一些局的游戏已经结束，然后删除

        // 遍历所有游戏，先更新超时，再更新结束
        for (_, game) in match_game_map.iter_mut() {
            // 更新玩家操作超时状态
            game.update_opt_timeout_status(get_timestamp());
            // 更新游戏结束状态
            game.update_end_status();

            // 如果数据有变化，则同步给这局游戏的两个玩家
            if game.is_dirty() {
                // 同步游戏
                if let Some(proto_json_str) = game.gc_to_json() {
                    handler.signals().send(Signal::Sync(
                        game.player1.endpoint_id.clone(),
                        game.player2.endpoint_id.clone(),
                        proto_json_str,
                    ));
                }
            }

            // 如果这局游戏结束，准备从游戏字典里删除这局游戏
            if game.is_game_end() {
                // 准备删除这个游戏
                ended_game.push(game.id.clone());
            }
        }

        // 正式删除已经结束的游戏局
        for game_id in ended_game.iter() {
            match_game_map.remove(game_id);
        }

        // 遍历匹配队列，对达到匹配条件的玩家建立游戏
    }

    // server_task.join().unwrap();
    // client_handle.join().unwrap();
}

fn get_timestamp() -> i64 {
    Utc::now().timestamp()
}
