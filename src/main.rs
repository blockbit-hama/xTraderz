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
mod market_data_publisher;
mod util;

use warp::Filter;
use tokio::sync::mpsc;
use std::sync::Arc;
use chrono::Utc;

use crate::models::{OrderMessage, Execution, OrderBook, Side, OrderType, OrderStatus, Order};
use crate::websocket::execution_push::ExecutionPushManager;
use crate::market_data_publisher::publisher::MarketDataPublisher;

#[tokio::main]
async fn main() {
  // 기본 채널 생성 - 주문 처리 흐름
  let (order_tx, order_rx) = mpsc::channel(100);
  let (exec_tx, mut exec_rx) = mpsc::channel(100);
  
  // WebSocket 관리자 생성 (실시간 체결 정보 푸시)
  let execution_push_manager = Arc::new(ExecutionPushManager::new());
  
  // 시장 데이터 발행자 (MDP) 생성
  let market_data_publisher = Arc::new(MarketDataPublisher::new());
  let mdp_clone = market_data_publisher.clone();
  
  // 오더북 상태 저장소 생성
  let orderbook_store = Arc::new(tokio::sync::Mutex::new(OrderBook::new()));
  let orderbook_store_for_mdp = orderbook_store.clone();
  
  // 시퀀서 실행 (주문 처리 파이프라인)
  tokio::spawn(async move {
    sequencer::run(order_rx, exec_tx).await;
  });
  
  // 체결 수신 및 분배
  let exec_push_manager_clone = execution_push_manager.clone();
  let mdp_for_exec = market_data_publisher.clone();
  tokio::spawn(async move {
    while let Some(exec) = exec_rx.recv().await {
      println!("체결 발생: 심볼 = {}, 가격 = {}, 수량 = {}",
               exec.symbol, exec.price, exec.quantity);
      
      // 1. WebSocket을 통해 체결 정보 브로드캐스트 (실시간 알림)
      exec_push_manager_clone.broadcast_execution(&exec).await;
      
      // 2. 오더북 상태 업데이트 (체결에 따른 변경)
      let mut orderbook_guard = orderbook_store.lock().await;
      // 실제로는 체결에 따른 오더북 상태 변경 로직이 필요합니다.
      // 예시 로직: orderbook_guard.update_after_execution(&exec);
      
      // 3. 시장 데이터 발행자에 오더북 상태 및 체결 정보 전달
      let orderbook_clone = orderbook_guard.clone();
      let symbol = exec.symbol.clone();
      
      // 락을 해제한 후 MDP 호출
      drop(orderbook_guard);
      
      // 오더북 상태 업데이트
      mdp_for_exec.update_orderbook(&symbol, orderbook_clone);
      
      // 체결 정보 처리 (캔들스틱, 시장 통계 등 업데이트)
      mdp_for_exec.process_execution(&exec);
    }
  });
  
  // REST API 라우트 - 주문 관리자
  let api_routes = order_manager::routes(order_tx.clone(), Arc::new(tokio::sync::Mutex::new(Vec::new())));
  
  // WebSocket 라우트 - 체결 정보
  let ws_exec_routes = websocket::execution_push::ws_execution_route(
    execution_push_manager.clone()
  );
  
  // 시장 데이터 API 라우트
  let market_data_routes = market_data_publisher.routes();
  
  // 모든 라우트 합치기
  let routes = api_routes
    .or(ws_exec_routes)
    .or(market_data_routes)
    .with(warp::cors().allow_any_origin());
  
  // 서버 시작
  println!("주문 매칭 엔진이 http://127.0.0.1:3030 에서 시작합니다");
  println!("API 엔드포인트:");
  println!("  - 주문 생성/취소: POST /v1/order, POST /v1/order/cancel");
  println!("  - 체결 WebSocket: ws://127.0.0.1:3030/ws/executions");
  println!("  - 시장 데이터 API: ");
  println!("      GET /api/v1/orderbook/{symbol}");
  println!("      GET /api/v1/executions/{symbol}");
  println!("      GET /api/v1/statistics/{symbol}");
  println!("      GET /api/v1/klines/{symbol}/{interval}");
  
  warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}