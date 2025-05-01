/**
* filename : simple_client
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use futures::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use serde_json::{json, Value};
use std::time::Duration;
use std::error::Error;
use reqwest;

/// 간단한 클라이언트 예제
/// 이 예제는 주문 매칭 엔진 서버에 REST API를 통해 주문을 제출하고
/// 체결 발생 시 WebSocket을 통해 알림을 받으며
/// Market Data Publisher(MDP)의 API를 통해 시장 데이터를 조회합니다.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // 서버 URL 설정
  let server_base = "http://127.0.0.1:3030";
  let ws_base = "ws://127.0.0.1:3030";
  
  // HTTP 클라이언트 생성
  let client = reqwest::Client::new();
  
  println!("주문 매칭 엔진 클라이언트 예제");
  println!("------------------------------");
  
  // 체결 수신용 WebSocket 연결
  let executions_url = Url::parse(&format!("{}/ws/executions", ws_base))?;
  let (executions_ws, _) = connect_async(executions_url).await?;
  let (mut exec_write, mut exec_read) = executions_ws.split();
  
  println!("체결 스트림에 연결됨");
  
  // 체결 메시지 처리 작업
  let exec_task = tokio::spawn(async move {
    while let Some(msg) = exec_read.next().await {
      match msg {
        Ok(Message::Text(text)) => {
          let json: Value = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(e) => {
              eprintln!("체결 JSON 파싱 오류: {}", e);
              continue;
            }
          };
          
          println!("체결 발생: {} - 가격: {}, 수량: {}",
                   json["symbol"], format_number(json["price"].as_u64().unwrap_or(0)), json["quantity"]);
        },
        Ok(Message::Close(_)) => break,
        Err(e) => {
          eprintln!("체결 스트림 오류: {}", e);
          break;
        }
        _ => {}
      }
    }
  });
  
  // 시장 데이터 조회 - 현재 오더북 상태
  let resp = client.get(&format!("{}/api/v1/orderbook/BTC-KRW", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let orderbook: Value = resp.json().await?;
    println!("\n현재 BTC-KRW 오더북:");
    
    println!("매도 호가:");
    if let Some(asks) = orderbook["asks"].as_array() {
      for (i, ask) in asks.iter().take(5).enumerate() {
        println!("  #{}: 가격 {}, 수량 {}",
                 i+1, format_number(ask["price"].as_u64().unwrap_or(0)), ask["volume"]);
      }
    }
    
    println!("매수 호가:");
    if let Some(bids) = orderbook["bids"].as_array() {
      for (i, bid) in bids.iter().take(5).enumerate() {
        println!("  #{}: 가격 {}, 수량 {}",
                 i+1, format_number(bid["price"].as_u64().unwrap_or(0)), bid["volume"]);
      }
    }
  } else {
    println!("오더북 조회 실패: {}", resp.status());
  }
  
  // 매수 주문 제출
  println!("\n매수 주문 제출 중...");
  let buy_order = json!({
        "symbol": "BTC-KRW",
        "side": "Buy",
        "price": 50000000,
        "order_type": "Limit",
        "quantity": 1
    });
  
  let resp = client.post(&format!("{}/v1/order", server_base))
    .json(&buy_order)
    .send()
    .await?;
  
  let buy_order_id: String;
  
  if resp.status().is_success() {
    let order: Value = resp.json().await?;
    buy_order_id = order["order_id"].as_str().unwrap_or("").to_string();
    println!("매수 주문 생성됨: ID = {}", buy_order_id);
  } else {
    eprintln!("매수 주문 생성 실패: {}", resp.status());
    buy_order_id = String::new();
  }
  
  // 잠시 대기
  tokio::time::sleep(Duration::from_secs(1)).await;
  
  // 매도 주문 제출
  println!("\n매도 주문 제출 중...");
  let sell_order = json!({
        "symbol": "BTC-KRW",
        "side": "Sell",
        "price": 50000000,  // 동일 가격으로 매칭되도록
        "order_type": "Limit",
        "quantity": 1
    });
  
  let resp = client.post(&format!("{}/v1/order", server_base))
    .json(&sell_order)
    .send()
    .await?;
  
  if resp.status().is_success() {
    let order: Value = resp.json().await?;
    println!("매도 주문 생성됨: ID = {}", order["order_id"]);
  } else {
    eprintln!("매도 주문 생성 실패: {}", resp.status());
  }
  
  // 체결이 발생할 시간 여유를 두고 대기
  println!("\n체결 대기 중...");
  tokio::time::sleep(Duration::from_secs(3)).await;
  
  // MDP API를 통한 체결 내역 조회
  println!("\n체결 내역 조회 중...");
  let resp = client.get(&format!("{}/api/v1/executions/BTC-KRW", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let executions: Vec<Value> = resp.json().await?;
    println!("체결 내역 수신: {} 건", executions.len());
    
    for (i, exec) in executions.iter().enumerate().take(5) {
      println!("#{}: 주문 ID: {}, 가격: {}, 수량: {}",
               i+1, exec["order_id"], format_number(exec["price"].as_u64().unwrap_or(0)), exec["volume"]);
    }
  } else {
    eprintln!("체결 내역 조회 실패: {}", resp.status());
  }
  
  // 시장 통계 조회
  println!("\n시장 통계 조회 중...");
  let resp = client.get(&format!("{}/api/v1/statistics/BTC-KRW", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let stats: Value = resp.json().await?;
    println!("BTC-KRW 시장 통계:");
    println!("마지막 가격: {}", format_number(stats["last_price"].as_u64().unwrap_or(0)));
    println!("24시간 고가: {}", format_number(stats["high_price_24h"].as_u64().unwrap_or(0)));
    println!("24시간 저가: {}", format_number(stats["low_price_24h"].as_u64().unwrap_or(0)));
    println!("24시간 거래량: {}", stats["volume_24h"]);
    println!("24시간 변동률: {}%", stats["price_change_24h"]);
  } else {
    eprintln!("시장 통계 조회 실패: {}", resp.status());
  }
  
  // 캔들스틱 데이터 조회
  println!("\n캔들스틱 데이터 조회 중...");
  let resp = client.get(&format!("{}/api/v1/klines/BTC-KRW/1m", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let candles: Vec<Value> = resp.json().await?;
    println!("BTC-KRW 1분봉 데이터: {} 개", candles.len());
    
    for (i, candle) in candles.iter().enumerate().take(3) {
      println!("#{}: 시간: {}, 시가: {}, 고가: {}, 저가: {}, 종가: {}, 거래량: {}",
               i+1,
               candle["open_time"],
               format_number(candle["open"].as_u64().unwrap_or(0)),
               format_number(candle["high"].as_u64().unwrap_or(0)),
               format_number(candle["low"].as_u64().unwrap_or(0)),
               format_number(candle["close"].as_u64().unwrap_or(0)),
               candle["volume"]);
    }
  } else {
    eprintln!("캔들스틱 데이터 조회 실패: {}", resp.status());
  }
  
  // 프로그램 종료 전에 잠시 대기
  println!("\n프로그램 종료 예정 (3초 후)...");
  tokio::time::sleep(Duration::from_secs(3)).await;
  
  // 작업 정리
  exec_task.abort();
  
  Ok(())
}

/// 숫자 포맷팅 헬퍼 함수 (천 단위 구분자 추가)
fn format_number(num: u64) -> String {
  let num_str = num.to_string();
  let mut result = String::new();
  let len = num_str.len();
  
  for (i, c) in num_str.chars().enumerate() {
    if i > 0 && (len - i) % 3 == 0 {
      result.push(',');
    }
    result.push(c);
  }
  
  result
}