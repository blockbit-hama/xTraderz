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

/// 간단한 WebSocket 클라이언트 예제
/// 이 예제는 주문 매칭 엔진 서버에 REST API를 통해 주문을 제출하고
/// WebSocket을 통해 체결 정보와 오더북 업데이트를 수신합니다.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // 서버 URL 설정
  let server_base = "http://127.0.0.1:3030";
  let ws_base = "ws://127.0.0.1:3030";
  
  // HTTP 클라이언트 생성
  let client = reqwest::Client::new();
  
  println!("간단한 주문 매칭 엔진 클라이언트 예제");
  println!("---------------------------------------");
  
  // 체결 수신용 WebSocket 연결
  let executions_url = Url::parse(&format!("{}/ws/executions", ws_base))?;
  let (executions_ws, _) = connect_async(executions_url).await?;
  let (mut exec_write, mut exec_read) = executions_ws.split();
  
  println!("체결 스트림에 연결됨");
  
  // 오더북 수신용 WebSocket 연결
  let orderbook_url = Url::parse(&format!("{}/ws/orderbook/BTC-KRW", ws_base))?;
  let (orderbook_ws, _) = connect_async(orderbook_url).await?;
  let (mut ob_write, mut ob_read) = orderbook_ws.split();
  
  println!("오더북 스트림에 연결됨 (BTC-KRW)");
  
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
          
          println!("체결 수신: {} - 가격: {}, 수량: {}",
                   json["symbol"], json["price"], json["quantity"]);
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
  
  // 오더북 메시지 처리 작업
  let ob_task = tokio::spawn(async move {
    while let Some(msg) = ob_read.next().await {
      match msg {
        Ok(Message::Text(text)) => {
          let json: Value = match serde_json::from_str(&text) {
            Ok(json) => json,
            Err(e) => {
              eprintln!("오더북 JSON 파싱 오류: {}", e);
              continue;
            }
          };
          
          let msg_type = json["type"].as_str().unwrap_or("unknown");
          
          if msg_type == "orderbook_snapshot" {
            println!("오더북 스냅샷 수신");
            print_orderbook(&json["data"]);
          } else if msg_type == "orderbook" {
            println!("오더북 업데이트 수신");
            print_orderbook(&json["data"]);
          }
        },
        Ok(Message::Close(_)) => break,
        Err(e) => {
          eprintln!("오더북 스트림 오류: {}", e);
          break;
        }
        _ => {}
      }
    }
  });
  
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
  
  if resp.status().is_success() {
    let order: Value = resp.json().await?;
    println!("매수 주문 생성됨: ID = {}", order["order_id"]);
  } else {
    eprintln!("매수 주문 생성 실패: {}", resp.status());
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
  
  // 체결 내역 조회
  println!("\n체결 내역 조회 중...");
  let resp = client.get(&format!("{}/v1/execution?symbol=BTC-KRW", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let executions: Vec<Value> = resp.json().await?;
    println!("체결 내역 수신: {} 건", executions.len());
    
    for (i, exec) in executions.iter().enumerate() {
      println!("#{}: 주문 ID: {}, 가격: {}, 수량: {}",
               i+1, exec["order_id"], exec["price"], exec["quantity"]);
    }
  } else {
    eprintln!("체결 내역 조회 실패: {}", resp.status());
  }
  
  // 프로그램 종료 전에 잠시 대기
  println!("\n프로그램 종료 예정 (3초 후)...");
  tokio::time::sleep(Duration::from_secs(3)).await;
  
  // 작업 정리
  exec_task.abort();
  ob_task.abort();
  
  Ok(())
}

/// 오더북 출력 헬퍼 함수
fn print_orderbook(orderbook: &Value) {
  println!("심볼: {}", orderbook["symbol"]);
  
  println!("매도호가:");
  if let Some(asks) = orderbook["asks"].as_array() {
    for (i, ask) in asks.iter().enumerate().take(5) {
      println!("  #{}: 가격 {}, 수량 {}",
               i+1, ask["price"], ask["volume"]);
    }
  }
  
  println!("매수호가:");
  if let Some(bids) = orderbook["bids"].as_array() {
    for (i, bid) in bids.iter().enumerate().take(5) {
      println!("  #{}: 가격 {}, 수량 {}",
               i+1, bid["price"], bid["volume"]);
    }
  }
}