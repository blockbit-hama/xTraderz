/**
* filename : mdp_test
* author : HAMA
* date: 2025. 5. 1.
* description: 
**/

use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;
use warp::test::request;
use warp::http::StatusCode;
use serde_json::Value;

use order_matching_engine::models::{Order, Execution, Side, OrderType, OrderStatus, OrderBook};
use order_matching_engine::market_data_publisher::publisher::MarketDataPublisher;
use order_matching_engine::market_data_publisher::models::CandleInterval;

#[tokio::test]
async fn test_mdp_orderbook_api() {
  // MDP 인스턴스 생성
  let mdp = MarketDataPublisher::new();
  
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
    price: 51000000,
    quantity: 3,
    side: Side::Sell,
    order_type: OrderType::Limit,
    status: OrderStatus::New,
    filled_quantity: 0,
    remain_quantity: 3,
    entry_time: Utc::now(),
  };
  orderbook.insert_order(sell_order);
  
  // MDP에 오더북 추가
  mdp.update_orderbook("BTC-KRW", orderbook);
  
  // API 라우트 생성
  let routes = mdp.routes();
  
  // 오더북 API 요청
  let resp = request()
    .method("GET")
    .path("/api/v1/orderbook/BTC-KRW")
    .reply(&routes)
    .await;
  
  // 응답 검증
  assert_eq!(resp.status(), StatusCode::OK);
  
  // JSON 파싱
  let body = String::from_utf8(resp.body().to_vec()).unwrap();
  let json: Value = serde_json::from_str(&body).unwrap();
  
  // 오더북 필드 검증
  assert_eq!(json["symbol"], "BTC-KRW");
  
  // 매수/매도 호가 검증
  let bids = json["bids"].as_array().unwrap();
  let asks = json["asks"].as_array().unwrap();
  
  assert_eq!(bids.len(), 1);
  assert_eq!(asks.len(), 1);
  
  assert_eq!(bids[0]["price"], 50000000);
  assert_eq!(bids[0]["volume"], 5);
  
  assert_eq!(asks[0]["price"], 51000000);
  assert_eq!(asks[0]["volume"], 3);
}

#[tokio::test]
async fn test_mdp_execution_processing() {
  // MDP 인스턴스 생성
  let mdp = MarketDataPublisher::new();
  
  // 테스트 체결 생성
  let execution = Execution {
    exec_id: "exec_1".to_string(),
    order_id: "order_1".to_string(),
    symbol: "ETH-KRW".to_string(),
    side: Side::Buy,
    price: 2000000,
    quantity: 2,
    fee: 0.05,
    transaction_time: Utc::now(),
  };
  
  // 체결 처리
  mdp.process_execution(&execution);
  
  // API 라우트 생성
  let routes = mdp.routes();
  
  // 체결 내역 API 요청
  let resp = request()
    .method("GET")
    .path("/api/v1/executions/ETH-KRW")
    .reply(&routes)
    .await;
  
  // 응답 검증
  assert_eq!(resp.status(), StatusCode::OK);
  
  // JSON 파싱
  let body = String::from_utf8(resp.body().to_vec()).unwrap();
  let json: Value = serde_json::from_str(&body).unwrap();
  
  // 체결 내역 검증
  assert!(json.is_array());
  let executions = json.as_array().unwrap();
  assert_eq!(executions.len(), 1);
  
  let exec = &executions[0];
  assert_eq!(exec["symbol"], "ETH-KRW");
  assert_eq!(exec["price"], 2000000);
  assert_eq!(exec["volume"], 2);
  
  // 시장 통계 API 요청
  let resp = request()
    .method("GET")
    .path("/api/v1/statistics/ETH-KRW")
    .reply(&routes)
    .await;
  
  // 응답 검증
  assert_eq!(resp.status(), StatusCode::OK);
  
  // JSON 파싱
  let body = String::from_utf8(resp.body().to_vec()).unwrap();
  let json: Value = serde_json::from_str(&body).unwrap();
  
  // 시장 통계 검증
  assert_eq!(json["symbol"], "ETH-KRW");
  assert_eq!(json["open_price_24h"], 2000000);
  assert_eq!(json["high_price_24h"], 2000000);
  assert_eq!(json["low_price_24h"], 2000000);
  assert_eq!(json["last_price"], 2000000);
  assert_eq!(json["volume_24h"], 2);
}

#[tokio::test]
async fn test_mdp_candlestick() {
  // MDP 인스턴스 생성
  let mdp = MarketDataPublisher::new();
  
  // 첫 번째 체결 생성 및 처리
  let execution1 = Execution {
    exec_id: "exec_1".to_string(),
    order_id: "order_1".to_string(),
    symbol: "BTC-KRW".to_string(),
    side: Side::Buy,
    price: 50000000,
    quantity: 1,
    fee: 0.05,
    transaction_time: Utc::now(),
  };
  
  mdp.process_execution(&execution1);
  
  // 잠시 대기 후 두 번째 체결 생성 및 처리
  sleep(Duration::from_secs(1)).await;
  
  let execution2 = Execution {
    exec_id: "exec_2".to_string(),
    order_id: "order_2".to_string(),
    symbol: "BTC-KRW".to_string(),
    side: Side::Sell,
    price: 50100000,
    quantity: 2,
    fee: 0.05,
    transaction_time: Utc::now(),
  };
  
  mdp.process_execution(&execution2);
  
  // API 라우트 생성
  let routes = mdp.routes();
  
  // 1분봉 데이터 API 요청
  let resp = request()
    .method("GET")
    .path("/api/v1/klines/BTC-KRW/1m")
    .reply(&routes)
    .await;
  
  // 응답 검증
  assert_eq!(resp.status(), StatusCode::OK);
  
  // JSON 파싱
  let body = String::from_utf8(resp.body().to_vec()).unwrap();
  let json: Value = serde_json::from_str(&body).unwrap();
  
  // 캔들스틱 데이터 검증
  assert!(json.is_array());
  let candles = json.as_array().unwrap();
  assert!(!candles.is_empty());
  
  let candle = &candles[0];
  assert_eq!(candle["symbol"], "BTC-KRW");
  assert_eq!(candle["interval"], "1m");
  
  // OHLCV 데이터 검증
  assert_eq!(candle["open"], 50000000);
  assert_eq!(candle["high"], 50100000);
  assert_eq!(candle["low"], 50000000);
  assert_eq!(candle["close"], 50100000);
  assert_eq!(candle["volume"], 3);
  assert_eq!(candle["trade_count"], 2);
  
  // 잘못된 간격 요청
  let resp = request()
    .method("GET")
    .path("/api/v1/klines/BTC-KRW/2m")
    .reply(&routes)
    .await;
  
  // 응답 검증
  assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}