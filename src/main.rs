/**
* filename : main
* author : HAMA
* date: 2025. 4. 10.
* description: HTTP API 서버 구현
**/

mod models;
mod matching_engine;
mod sequencer;
mod order_manager;
mod websocket;
mod relay;
mod util;

use warp::Filter;
use tokio::sync::mpsc;
use std::sync::Arc;
use chrono::Utc;

use crate::models::{OrderMessage, Execution, OrderBook, Side, OrderType, OrderStatus, Order};
use crate::websocket::{execution_push::ExecutionPushManager, orderbook_relay::OrderBookRelayManager};
use crate::relay::{server::RelayServer, client_handler::{ClientMessage, ClientHandler}};

#[tokio::main]
async fn main() {
  // 기본 채널 생성 - 주문 처리 흐름
  let (order_tx, order_rx) = mpsc::channel(100);
  let (exec_tx, mut exec_rx) = mpsc::channel(100);
  
  // WebSocket 관리자 생성
  let execution_push_manager = Arc::new(ExecutionPushManager::new());
  let orderbook_relay_manager = Arc::new(OrderBookRelayManager::new());
  
  // 클라이언트 메시지 채널 생성
  let (client_msg_tx, client_msg_rx) = mpsc::channel(100);
  
  // 실행 저장소 생성
  let exec_store = Arc::new(tokio::sync::Mutex::new(Vec::new()));
  let store_clone = exec_store.clone();
  
  // 오더북 상태 저장소 생성 (릴레이 서버와 공유)
  let orderbook_store = Arc::new(tokio::sync::Mutex::new(OrderBook::new()));
  let orderbook_store_for_relay = orderbook_store.clone();
  
  // 시퀀서 실행 (주문 처리 파이프라인)
  tokio::spawn(async move {
    sequencer::run(order_rx, exec_tx).await;
  });
  
  // 릴레이 서버 생성 및 실행 (오더북 UI 업데이트용)
  let (relay_server, client_update_rx) = RelayServer::new();
  let relay_server_clone = Arc::new(relay_server);
  
  // 릴레이 서버 작업
  let relay_server_for_run = relay_server_clone.clone();
  tokio::spawn(async move {
    relay_server_for_run.run(client_msg_rx).await;
  });
  
  // 체결 수신 및 분배
  let exec_push_manager_clone = execution_push_manager.clone();
  let relay_server_for_orderbook = relay_server_clone.clone();
  tokio::spawn(async move {
    while let Some(exec) = exec_rx.recv().await {
      // 1. 저장소에 저장
      store_clone.lock().await.push(exec.clone());
      
      // 2. WebSocket을 통해 체결 정보 브로드캐스트
      exec_push_manager_clone.broadcast_execution(&exec).await;
      
      // 3. 오더북 상태 업데이트 (체결에 따른 변경)
      let mut orderbook = orderbook_store.lock().await;
      // 실제로는 체결에 따른 오더북 상태 변경 로직이 필요합니다.
      // 예시 로직: orderbook.update_after_execution(&exec);
      
      // 4. 릴레이 서버에 오더북 상태 전달
      let symbol = exec.symbol.clone();
      relay_server_for_orderbook.update_orderbook(&symbol, orderbook.clone());
    }
  });
  
  // 오더북 업데이트 브로드캐스터
  let ob_relay_manager_clone = orderbook_relay_manager.clone();
  tokio::spawn(async move {
    websocket::orderbook_relay::run_orderbook_broadcaster(ob_relay_manager_clone).await;
  });
  
  // REST API 라우트
  let api_routes = order_manager::routes(order_tx.clone(), exec_store.clone());
  
  // WebSocket 라우트
  let ws_exec_routes = websocket::execution_push::ws_execution_route(
    execution_push_manager.clone()
  );
  
  let ws_orderbook_routes = websocket::orderbook_relay::ws_orderbook_route(
    orderbook_relay_manager.clone()
  );
  
  // 모든 라우트 합치기
  let routes = api_routes
    .or(ws_exec_routes)
    .or(ws_orderbook_routes)
    .with(warp::cors().allow_any_origin());
  
  // 서버 시작
  println!("주문 매칭 엔진이 http://127.0.0.1:3030 에서 시작합니다");
  println!("WebSocket 엔드포인트:");
  println!("  - ws://127.0.0.1:3030/ws/executions");
  println!("  - ws://127.0.0.1:3030/ws/orderbook/{symbol}");
  
  warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}