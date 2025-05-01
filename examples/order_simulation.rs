/**
* filename : order_simulation
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use futures::{SinkExt, StreamExt};
use reqwest;
use serde_json::{json, Value};
use std::error::Error;
use std::time::Duration;
use tokio::time;
use rand::{thread_rng, Rng};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

/// 주문 시뮬레이션 예제
/// 이 예제는 주문 매칭 엔진에 다수의 주문을 연속적으로 제출하여
/// 실제 거래 환경을 시뮬레이션합니다. 시뮬레이션 중에는 체결 정보를
/// WebSocket으로 수신하고, Market Data Publisher API를 통해
/// 오더북과 캔들스틱 데이터를 정기적으로 조회합니다.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // 서버 URL 설정
  let server_base = "http://127.0.0.1:3030";
  let ws_base = "ws://127.0.0.1:3030";
  
  // HTTP 클라이언트 생성
  let client = reqwest::Client::new();
  
  println!("주문 시뮬레이션 시작");
  println!("--------------------");
  
  // 체결 수신용 WebSocket 연결
  let executions_url = Url::parse(&format!("{}/ws/executions", ws_base))?;
  let (executions_ws, _) = connect_async(executions_url).await?;
  let (mut exec_write, mut exec_read) = executions_ws.split();
  
  println!("체결 스트림에 연결됨");
  
  // 체결 모니터링 작업
  let exec_task = tokio::spawn(async move {
    let mut execution_count = 0;
    
    while let Some(msg) = exec_read.next().await {
      if let Ok(Message::Text(text)) = msg {
        let json: Value = match serde_json::from_str(&text) {
          Ok(json) => json,
          Err(_) => continue,
        };
        
        execution_count += 1;
        println!("\n체결 #{}: {} - 가격: {}, 수량: {}",
                 execution_count,
                 json["symbol"],
                 format_price(json["price"].as_u64().unwrap_or(0)),
                 json["quantity"]);
      }
    }
  });
  
  // 오더북 모니터링 작업
  let client_clone = client.clone();
  let orderbook_task = tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(5));
    
    loop {
      interval.tick().await;
      
      // 오더북 상태 조회
      match client_clone.get(&format!("{}/api/v1/orderbook/BTC-KRW", server_base))
        .send()
        .await {
        Ok(resp) if resp.status().is_success() => {
          if let Ok(orderbook) = resp.json::<Value>().await {
            let bids = orderbook["bids"].as_array().unwrap_or(&vec![]);
            let asks = orderbook["asks"].as_array().unwrap_or(&vec![]);
            
            // 최고 매수가와 최저 매도가 추출
            let top_bid = if !bids.is_empty() {
              bids[0]["price"].as_u64().unwrap_or(0)
            } else {
              0
            };
            
            let top_ask = if !asks.is_empty() {
              asks[0]["price"].as_u64().unwrap_or(0)
            } else {
              0
            };
            
            if top_bid > 0 && top_ask > 0 {
              println!("\n현재 시장 상태:");
              println!("최고 매수가: {}", format_price(top_bid));
              println!("최저 매도가: {}", format_price(top_ask));
              println!("스프레드: {}", format_price(top_ask.saturating_sub(top_bid)));
              println!("매수 단계: {}, 매도 단계: {}", bids.len(), asks.len());
            }
          }
        },
        _ => {}
      }
    }
  });
  
  // 시장 통계 모니터링 작업
  let client_clone = client.clone();
  let stats_task = tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(10));
    
    loop {
      interval.tick().await;
      
      // 시장 통계 조회
      match client_clone.get(&format!("{}/api/v1/statistics/BTC-KRW", server_base))
        .send()
        .await {
        Ok(resp) if resp.status().is_success() => {
          if let Ok(stats) = resp.json::<Value>().await {
            println!("\n24시간 시장 통계:");
            println!("마지막 가격: {}", format_price(stats["last_price"].as_u64().unwrap_or(0)));
            println!("24시간 고가: {}", format_price(stats["high_price_24h"].as_u64().unwrap_or(0)));
            println!("24시간 저가: {}", format_price(stats["low_price_24h"].as_u64().unwrap_or(0)));
            println!("24시간 거래량: {}", stats["volume_24h"]);
            println!("24시간 변동률: {}%", stats["price_change_24h"]);
          }
        },
        _ => {}
      }
    }
  });
  
  // 캔들스틱 모니터링 작업
  let client_clone = client.clone();
  let candle_task = tokio::spawn(async move {
    let mut interval = time::interval(Duration::from_secs(30));
    
    loop {
      interval.tick().await;
      
      // 1분봉 데이터 조회
      match client_clone.get(&format!("{}/api/v1/klines/BTC-KRW/1m?limit=1", server_base))
        .send()
        .await {
        Ok(resp) if resp.status().is_success() => {
          if let Ok(candles) = resp.json::<Vec<Value>>().await {
            if !candles.is_empty() {
              let candle = &candles[0];
              println!("\n최근 1분봉:");
              println!("시간: {}", candle["open_time"]);
              println!("시가: {}", format_price(candle["open"].as_u64().unwrap_or(0)));
              println!("고가: {}", format_price(candle["high"].as_u64().unwrap_or(0)));
              println!("저가: {}", format_price(candle["low"].as_u64().unwrap_or(0)));
              println!("종가: {}", format_price(candle["close"].as_u64().unwrap_or(0)));
              println!("거래량: {}", candle["volume"]);
              println!("거래 횟수: {}", candle["trade_count"]);
            }
          }
        },
        _ => {}
      }
    }
  });
  
  // 시뮬레이션 파라미터
  let simulation_time = Duration::from_secs(120); // 시뮬레이션 실행 시간
  let base_price = 50_000_000; // 기준 가격 (5천만원)
  let price_volatility = 0.02; // 가격 변동성 (±2%)
  let order_interval = Duration::from_millis(500); // 주문 간격
  
  let mut rng = thread_rng();
  let mut order_count = 0;
  let mut interval = time::interval(order_interval);
  let start_time = std::time::Instant::now();
  
  println!("\n주문 생성 시작 (약 {:.0}초 동안 실행)...", simulation_time.as_secs_f64());
  
  // 주문 생성 루프
  while start_time.elapsed() < simulation_time {
    interval.tick().await;
    
    // 무작위 가격 생성 (기준 가격에서 ±가격 변동성)
    let price_factor = 1.0 + price_volatility * (rng.gen::<f64>() * 2.0 - 1.0);
    let price = (base_price as f64 * price_factor).round() as u64;
    
    // 무작위 수량 (0.01 ~ 0.5 BTC)
    let quantity = (rng.gen::<f64>() * 0.49 + 0.01).round() * 100.0 / 100.0;
    
    // 매수/매도 무작위 선택
    let side = if rng.gen::<bool>() { "Buy" } else { "Sell" };
    
    let order = json!({
            "symbol": "BTC-KRW",
            "side": side,
            "price": price,
            "order_type": "Limit",
            "quantity": quantity
        });
    
    // 주문 제출
    let resp = client.post(&format!("{}/v1/order", server_base))
      .json(&order)
      .send()
      .await?;
    
    if resp.status().is_success() {
      let order_data: Value = resp.json().await?;
      order_count += 1;
      
      println!("주문 #{} 생성: {} {} BTC @ {} KRW",
               order_count,
               if side == "Buy" { "매수" } else { "매도" },
               quantity,
               format_price(price));
    } else {
      eprintln!("주문 생성 실패: {}", resp.status());
    }
    
    // 무작위로 기존 주문 취소 (25% 확률)
    if rng.gen::<f64>() < 0.25 && order_count > 0 {
      // 취소할 주문 ID 가져오기
      let order_id = order_data["order_id"].as_str().unwrap_or("");
      
      let cancel_req = json!({
                "order_id": order_id
            });
      
      let resp = client.post(&format!("{}/v1/order/cancel", server_base))
        .json(&cancel_req)
        .send()
        .await?;
      
      if resp.status().is_success() {
        println!("주문 취소: ID {}", order_id);
      }
    }
  }
  
  println!("\n시뮬레이션 완료: {} 주문 생성됨", order_count);
  
  // 최종 오더북 상태 확인 대기
  tokio::time::sleep(Duration::from_secs(2)).await;
  
  // 체결 내역 조회
  let resp = client.get(&format!("{}/api/v1/executions/BTC-KRW?limit=10", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let executions: Vec<Value> = resp.json().await?;
    println!("\n총 체결 수: {}", executions.len());
    
    println!("\n최근 체결 내역:");
    for (i, exec) in executions.iter().enumerate() {
      println!("#{}: 시간: {}, 가격: {}, 수량: {}, 방향: {}",
               i + 1,
               exec["timestamp"],
               format_price(exec["price"].as_u64().unwrap_or(0)),
               exec["volume"],
               exec["side"]);
    }
  }
  
  // 작업 정리
  println!("\n프로그램 종료...");
  exec_task.abort();
  orderbook_task.abort();
  stats_task.abort();
  candle_task.abort();
  
  Ok(())
}

/// 가격 포맷팅 헬퍼 함수
fn format_price(price: u64) -> String {
  let price_str = price.to_string();
  let mut formatted = String::new();
  let len = price_str.len();
  
  for (i, c) in price_str.chars().enumerate() {
    if i > 0 && (len - i) % 3 == 0 {
      formatted.push(',');
    }
    formatted.push(c);
  }
  
  format!("{} KRW", formatted)
}