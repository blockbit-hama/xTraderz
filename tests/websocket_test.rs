/**
* filename : websocket_test
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use warp::ws::Message;
use warp::Filter;
use warp::test::WsClient;
use futures::SinkExt;
use futures::StreamExt;
use serde_json::Value;
use chrono::Utc;

use xTraderz::models::{Order, Execution, Side, OrderType, OrderStatus};
use xTraderz::websocket::execution_push::ExecutionPushManager;
use xTraderz::websocket::orderbook_relay::OrderBookRelayManager;

#[tokio::test]
async fn test_execution_push() {
  // 실행 푸시 매니저 생성
  let manager = Arc::new(ExecutionPushManager::new());
  let manager_clone = manager.clone();
  
  // 웹소켓 라우트 생성
  let route = warp::path!("ws" / "executions")
    .and(warp::ws())
    .map(move |ws: warp::ws::Ws| {
      let manager = manager_clone.clone();
      ws.on_upgrade(move |socket| {
        order_matching_engine::websocket::execution_push::handle_execution_connection(socket, manager)
      })
    });
  
  // 테스트 서버 시작
  let (addr, server) = warp::serve(route)
    .bind_ephemeral(([127, 0, 0, 1], 0));
  
  let server_task = tokio::spawn(server);
  
  // WebSocket 클라이언트 생성
  let (ws_client, mut ws_rx) = warp::test::ws()
    .path("/ws/executions")
    .handshake(format!("ws://{}", addr))
    .await
    .expect("WebSocket 핸드셰이크 실패");
  
  // 체결 생성
  let execution = Execution {
    exec_id: "test_exec_1".to_string(),
    order_id: "test_order_1".to_string(),
    symbol: "BTC-KRW".to_string(),
    side: Side::Buy,
    price: 50000000,
    quantity: 1,
    fee: 0.05,
    transaction_time: Utc::now(),
  };
  
  // 체결 브로드캐스트
  manager.broadcast_execution(&execution).await;
  
  // 메시지 수신 대기
  tokio::time::sleep(Duration::from_millis(100)).await;
  
  let msg = ws_rx.next().await.expect("메시지 수신 실패").expect("WebSocket 오류");
  assert!(msg.is_text());
  
  // JSON 응답 검증
  let json: Value = serde_json::from_str(msg.to_str().unwrap()).expect("JSON 파싱 실패");
  assert_eq!(json["order_id"], "test_order_1");
  assert_eq!(json["symbol"], "BTC-KRW");
  
  // 정리
  drop(ws_client);
  server_task.abort();
}

#[tokio::test]
async fn test_orderbook_relay() {
  // 오더북 릴레이 매니저 생성
  let manager = Arc::new(OrderBookRelayManager::new());
  let manager_clone = manager.clone();
  
  // 테스트 오더북 생성
  let mut orderbook = order_matching_engine::models::OrderBook::new();
  
  // 매수 주문 추가
  let buy_order = Order {
    order_id: "buy_1".to_string(),
    symbol: "ETH-KRW".to_string(),
    price: 2000000,
    quantity: 5,
    side: Side::Buy,
    order_type: OrderType::Limit,
    status: OrderStatus::New,
    filled_quantity: 0,
    remain_quantity: 5,
    entry_time: Utc::now(),
  };
  orderbook.insert_order(buy_order);
  
  // 매도 주문 추가
  let sell_order = Order {
    order_id: "sell_1".to_string(),
    symbol: "ETH-KRW".to_string(),
    price: 2100000,
    quantity: 3,
    side: Side::Sell,
    order_type: OrderType::Limit,
    status: OrderStatus::New,
    filled_quantity: 0,
    remain_quantity: 3,
    entry_time: Utc::now(),
  };
  orderbook.insert_order(sell_order);
  
  // 오더북 업데이트
  manager.update_orderbook("ETH-KRW", orderbook);
  
  // 웹소켓 라우트 생성
  let route = warp::path!("ws" / "orderbook" / String)
    .and(warp::ws())
    .map(move |symbol: String, ws: warp::ws::Ws| {
      let manager = manager_clone.clone();
      ws.on_upgrade(move |socket| {
        order_matching_engine::websocket::orderbook_relay::handle_orderbook_connection(socket, symbol, manager)
      })
    });
  
  // 테스트 서버 시작
  let (addr, server) = warp::serve(route)
    .bind_ephemeral(([127, 0, 0, 1], 0));
  
  let server_task = tokio::spawn(server);
  
  // WebSocket 클라이언트 생성
  let (ws_client, mut ws_rx) = warp::test::ws()
    .path("/ws/orderbook/ETH-KRW")
    .handshake(format!("ws://{}", addr))
    .await
    .expect("WebSocket 핸드셰이크 실패");
  
  // 초기 스냅샷 수신 대기
  let msg = ws_rx.next().await.expect("메시지 수신 실패").expect("WebSocket 오류");
  assert!(msg.is_text());
  
  // JSON 응답 검증
  let json: Value = serde_json::from_str(msg.to_str().unwrap()).expect("JSON 파싱 실패");
  assert_eq!(json["type"], "orderbook_snapshot");
  
  let data = &json["data"];
  assert_eq!(data["symbol"], "ETH-KRW");
  
  // 매수/매도 주문 확인
  let bids = data["bids"].as_array().expect("bids가 배열이 아님");
  let asks = data["asks"].as_array().expect("asks가 배열이 아님");
  
  assert_eq!(bids.len(), 1);
  assert_eq!(asks.len(), 1);
  
  assert_eq!(bids[0]["price"], 2000000);
  assert_eq!(bids[0]["volume"], 5);
  
  assert_eq!(asks[0]["price"], 2100000);
  assert_eq!(asks[0]["volume"], 3);
  
  // 오더북 업데이트 브로드캐스트
  manager.broadcast_orderbook("ETH-KRW").await;
  
  // 업데이트 메시지 수신 대기
  let msg = ws_rx.next().await.expect("메시지 수신 실패").expect("WebSocket 오류");
  assert!(msg.is_text());
  
  // 업데이트 메시지 검증
  let json: Value = serde_json::from_str(msg.to_str().unwrap()).expect("JSON 파싱 실패");
  assert_eq!(json["type"], "orderbook");
  
  // 정리
  drop(ws_client);
  server_task.abort();
}