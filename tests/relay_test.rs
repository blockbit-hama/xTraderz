/**
* filename : relay_test
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use chrono::Utc;

use xTraderz::models::{Order, OrderBook, Side, OrderType, OrderStatus};
use xTraderz::relay::server::RelayServer;
use xTraderz::relay::client_handler::{ClientHandler, ClientMessage, ClientUpdate};
use xTraderz::util::serializer;

#[tokio::test]
async fn test_relay_server_orderbook_update() {
  // 릴레이 서버 생성
  let (relay_server, mut client_update_rx) = RelayServer::new();
  let relay_server = Arc::new(relay_server);
  
  // 클라이언트 메시지 채널 생성
  let (client_msg_tx, client_msg_rx) = mpsc::channel(100);
  
  // 릴레이 서버 실행
  let relay_server_clone = relay_server.clone();
  let server_task = tokio::spawn(async move {
    relay_server_clone.run(client_msg_rx).await;
  });
  
  // 테스트 오더북 생성
  let mut orderbook = OrderBook::new();
  
  // 매수 주문 추가
  let buy_order = Order {
    order_id: "buy_1".to_string(),
    symbol: "BTC-KRW".to_string(),
    price: 50000000,
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
    symbol: "BTC-KRW".to_string(),
    price: 52000000,
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
  relay_server.update_orderbook("BTC-KRW", orderbook);
  
  // 구독 메시지 전송
  client_msg_tx.send(ClientMessage::SubscribeOrderBook("BTC-KRW".to_string())).await.unwrap();
  
  // 충분한 시간 대기
  sleep(Duration::from_millis(100)).await;
  
  // 스냅샷 메시지 수신 확인
  let update = client_update_rx.recv().await.unwrap();
  
  match update {
    ClientUpdate::OrderBookSnapshot { symbol, data } => {
      assert_eq!(symbol, "BTC-KRW");
      assert_eq!(data.bids.len(), 1);
      assert_eq!(data.asks.len(), 1);
      assert_eq!(data.bids[0].price, 50000000);
      assert_eq!(data.asks[0].price, 52000000);
    },
    _ => panic!("잘못된 업데이트 유형"),
  }
  
  // 정리
  server_task.abort();
}

#[tokio::test]
async fn test_client_handler() {
  // 채널 생성
  let (client_tx, mut client_rx) = mpsc::channel(100);
  let (server_tx, mut server_rx) = mpsc::channel(100);
  
  // 클라이언트 핸들러 생성
  let client_handler = ClientHandler::new(client_tx, server_tx);
  
  // 구독 메시지 전송
  client_handler.handle_message("subscribe:ETH-KRW").await
    .expect("메시지 처리 실패");
  
  // 서버 측 메시지 수신
  let server_msg = server_rx.recv().await.expect("서버 메시지 수신 실패");
  
  // 메시지 검증
  match server_msg {
    ClientMessage::SubscribeOrderBook(symbol) => {
      assert_eq!(symbol, "ETH-KRW");
    },
    _ => panic!("잘못된 메시지 유형"),
  }
  
  // 스냅샷 요청 메시지 전송
  client_handler.handle_message("snapshot:BTC-KRW").await
    .expect("메시지 처리 실패");
  
  // 두 번째 서버 측 메시지 수신
  let server_msg = server_rx.recv().await.expect("서버 메시지 수신 실패");
  
  // 메시지 검증
  match server_msg {
    ClientMessage::RequestOrderBookSnapshot(symbol) => {
      assert_eq!(symbol, "BTC-KRW");
    },
    _ => panic!("잘못된 메시지 유형"),
  }
  
  // 구독 해제 메시지 전송
  client_handler.handle_message("unsubscribe:ETH-KRW").await
    .expect("메시지 처리 실패");
  
  // 세 번째 서버 측 메시지 수신
  let server_msg = server_rx.recv().await.expect("서버 메시지 수신 실패");
  
  // 메시지 검증
  match server_msg {
    ClientMessage::UnsubscribeOrderBook(symbol) => {
      assert_eq!(symbol, "ETH-KRW");
    },
    _ => panic!("잘못된 메시지 유형"),
  }
}

#[tokio::test]
async fn test_relay_server_multiple_clients() {
  // 릴레이 서버 생성
  let (relay_server, _) = RelayServer::new();
  let relay_server = Arc::new(relay_server);
  
  // 클라이언트 메시지 채널 생성
  let (client_msg_tx, client_msg_rx) = mpsc::channel(100);
  
  // 릴레이 서버 실행
  let relay_server_clone = relay_server.clone();
  let server_task = tokio::spawn(async move {
    relay_server_clone.run(client_msg_rx).await;
  });
  
  // 여러 클라이언트 구독 시뮬레이션
  let symbols = vec!["BTC-KRW", "ETH-KRW", "XRP-KRW"];
  
  for symbol in &symbols {
    // 구독 메시지 전송
    client_msg_tx.send(ClientMessage::SubscribeOrderBook(symbol.to_string())).await.unwrap();
    
    // 오더북 업데이트
    let mut orderbook = OrderBook::new();
    
    // 매수 주문 추가
    let buy_order = Order {
      order_id: format!("buy_{}", symbol),
      symbol: symbol.to_string(),
      price: 50000000,
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
      order_id: format!("sell_{}", symbol),
      symbol: symbol.to_string(),
      price: 52000000,
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
    relay_server.update_orderbook(symbol, orderbook);
  }
  
  // 검증: 해당 심볼에 대한 스냅샷 요청
  for symbol in &symbols {
    client_msg_tx.send(ClientMessage::RequestOrderBookSnapshot(symbol.to_string())).await.unwrap();
  }
  
  // 구독 해제 메시지 전송
  for symbol in &symbols {
    client_msg_tx.send(ClientMessage::UnsubscribeOrderBook(symbol.to_string())).await.unwrap();
  }
  
  // 충분한 시간 대기
  sleep(Duration::from_millis(100)).await;
  
  // 정리
  server_task.abort();
}