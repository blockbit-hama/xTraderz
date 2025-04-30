/**
* filename : orderbook_relay
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::watch;
use tokio::time::{self, Duration};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use serde_json::json;

use crate::models::{OrderBook, Book, Side, PriceLevel};
use crate::util::serializer::{orderbook_to_dto, PriceLevelDto};

// 오더북 중계 관리자
pub struct OrderBookRelayManager {
  connections: Arc<Mutex<HashMap<String, Vec<Sender<Message>>>>>,
  orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
}

impl OrderBookRelayManager {
  pub fn new() -> Self {
    OrderBookRelayManager {
      connections: Arc::new(Mutex::new(HashMap::new())),
      orderbooks: Arc::new(Mutex::new(HashMap::new())),
    }
  }
  
  // 오더북 업데이트
  pub fn update_orderbook(&self, symbol: &str, orderbook: OrderBook) {
    let mut books = self.orderbooks.lock().unwrap();
    books.insert(symbol.to_string(), orderbook);
  }
  
  // 오더북 스냅샷 생성
  fn create_snapshot(&self, symbol: &str) -> Option<serde_json::Value> {
    let books = self.orderbooks.lock().unwrap();
    let orderbook = books.get(symbol)?;
    
    // 오더북 DTO로 변환
    let dto = orderbook_to_dto(orderbook, symbol);
    
    // JSON으로 변환
    match serde_json::to_value(dto) {
      Ok(json) => Some(json),
      Err(_) => None,
    }
  }
  
  // 심볼에 새 연결 추가
  pub fn add_connection(&self, symbol: &str, tx: Sender<Message>) {
    let mut connections = self.connections.lock().unwrap();
    let symbol_connections = connections.entry(symbol.to_string()).or_insert_with(Vec::new);
    symbol_connections.push(tx);
  }
  
  // 연결 제거
  pub fn remove_connection(&self, symbol: &str, tx: &Sender<Message>) {
    let mut connections = self.connections.lock().unwrap();
    if let Some(symbol_connections) = connections.get_mut(symbol) {
      if let Some(pos) = symbol_connections.iter().position(|x| x.same_channel(tx)) {
        symbol_connections.remove(pos);
      }
    }
  }
  
  // 특정 심볼 오더북 브로드캐스트
  pub async fn broadcast_orderbook(&self, symbol: &str) {
    let snapshot = match self.create_snapshot(symbol) {
      Some(s) => s,
      None => return, // 오더북이 없으면 중단
    };
    
    let message = json!({
            "type": "orderbook",
            "data": snapshot
        });
    
    let message_json = match serde_json::to_string(&message) {
      Ok(j) => j,
      Err(_) => return, // 직렬화 에러
    };
    
    let connections = self.connections.lock().unwrap();
    if let Some(symbol_connections) = connections.get(symbol) {
      for tx in symbol_connections {
        let _ = tx.send(Message::text(message_json.clone())).await;
      }
    }
  }
}

// 오더북 WebSocket 라우트
pub fn ws_orderbook_route(
  manager: Arc<OrderBookRelayManager>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path!("ws" / "orderbook" / String)
    .and(warp::ws())
    .map(move |symbol: String, ws: warp::ws::Ws| {
      let manager = manager.clone();
      ws.on_upgrade(move |socket| handle_orderbook_connection(socket, symbol, manager))
    })
}

// 오더북 WebSocket 연결 처리
async fn handle_orderbook_connection(
  ws: WebSocket,
  symbol: String,
  manager: Arc<OrderBookRelayManager>
) {
  let (ws_tx, mut ws_rx) = ws.split();
  
  // 토키오 채널 생성
  let (tx, rx) = tokio::sync::mpsc::channel::<Message>(100);
  let manager_clone = manager.clone();
  let symbol_clone = symbol.clone();
  
  // 메시지 전송 작업
  tokio::task::spawn(rx.forward(ws_tx).map(|result| {
    if let Err(e) = result {
      eprintln!("WebSocket send error: {}", e);
    }
  }));
  
  // 연결 등록
  manager.add_connection(&symbol, tx.clone());
  
  // 초기 스냅샷 전송
  if let Some(snapshot) = manager.create_snapshot(&symbol) {
    let initial_message = json!({
            "type": "orderbook_snapshot",
            "data": snapshot
        });
    
    if let Ok(json) = serde_json::to_string(&initial_message) {
      let _ = tx.send(Message::text(json)).await;
    }
  }
  
  // 클라이언트 메시지 처리
  while let Some(result) = ws_rx.next().await {
    match result {
      Ok(msg) => {
        if msg.is_close() {
          break;
        }
        // 필요시 클라이언트 메시지 처리
      }
      Err(_) => {
        break;
      }
    }
  }
  
  // 연결 종료 정리
  manager_clone.remove_connection(&symbol_clone, &tx);
}

// 정기적 오더북 업데이트 작업
pub async fn run_orderbook_broadcaster(manager: Arc<OrderBookRelayManager>) {
  let mut interval = time::interval(Duration::from_millis(100)); // 100ms마다 업데이트
  
  loop {
    interval.tick().await;
    
    // 각 심볼에 대한 오더북 브로드캐스트
    let symbols = {
      let connections = manager.connections.lock().unwrap();
      connections.keys().cloned().collect::<Vec<String>>()
    };
    
    for symbol in symbols {
      manager.broadcast_orderbook(&symbol).await;
    }
  }
}