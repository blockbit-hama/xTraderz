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
/// 실제 거래 환경을 시뮬레이션합니다.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // 서버 URL 설정
  let server_base = "http://127.0.0.1:3030";
  let ws_base = "ws://127.0.0.1:3030";
  
  // HTTP 클라이언트 생성
  let client = reqwest::Client::new();
  
  println!("주문 시뮬레이션 시작");
  println!("--------------------");
  
  // 오더북 WebSocket 연결
  let orderbook_url = Url::parse(&format!("{}/ws/orderbook/BTC-KRW", ws_base))?;
  let (orderbook_ws, _) = connect_async(orderbook_url).await?;
  let (mut ob_write, mut ob_read) = orderbook_ws.split();
  
  println!("오더북 스트림에 연결됨 (BTC-KRW)");
  
  // 오더북 모니터링 작업
  let ob_task = tokio::spawn(async move {
    let mut prev_bid_price = 0;
    let mut prev_ask_price = u64::MAX;
    
    while let Some(msg) = ob_read.next().await {
      if let Ok(Message::Text(text)) = msg {
        let json: Value = match serde_json::from_str(&text) {
          Ok(json) => json,
          Err(_) => continue,
        };
        
        // 최고 매수가 및 최저 매도가 추출
        let data = match json.get("data") {
          Some(data) => data,
          None => continue,
        };
        
        // 매수 호가 확인
        let bids = match data["bids"].as_array() {
          Some(bids) if !bids.is_empty() => bids,
          _ => continue,
        };
        
        // 매도 호가 확인
        let asks = match data["asks"].as_array() {
          Some(asks) if !asks.is_empty() => asks,
          _ => continue,
        };
        
        // 최고 매수가
        let top_bid = match bids[0]["price"].as_u64() {
          Some(price) => price,
          None => continue,
        };
        
        // 최저 매도가
        let top_ask = match asks[0]["price"].as_u64() {
          Some(price) => price,
          None => continue,
        };
        
        // 변화가 있을 때만 출력
        if top_bid != prev_bid_price || top_ask != prev_ask_price {
          println!("\n현재 시장 상태:");
          println!("최고 매수가: {}", format_price(top_bid));
          println!("최저 매도가: {}", format_price(top_ask));
          println!("스프레드: {}", format_price(top_ask.saturating_sub(top_bid)));
          
          prev_bid_price = top_bid;
          prev_ask_price = top_ask;
        }
      }
    }
  });
  
  // 체결 모니터링용 WebSocket 연결
  let executions_url = Url::parse(&format!("{}/ws/executions", ws_base))?;
  let (executions_ws, _) = connect_async(executions_url).await?;
  let (mut exec_write, mut exec_read) = executions_ws.split();
  
  println!("체결 스트림에 연결됨");
  
  // 체결 모니터링 작업
  let exec_task = tokio::spawn(async move {
    while let Some(msg) = exec_read.next().await {
      if let Ok(Message::Text(text)) = msg {
        let json: Value = match serde_json::from_str(&text) {
          Ok(json) => json,
          Err(_) => continue,
        };
        
        println!("\n체결 발생: 가격 {}, 수량 {}, 주문 ID: {}",
                 format_price(json["price"].as_u64().unwrap_or(0)),
                 json["quantity"],
                 json["order_id"]);
      }
    }
  });
  
  // 시뮬레이션 파라미터
  let simulation_time = Duration::from_secs(60); // 시뮬레이션 실행 시간
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
      // 취소할 주문 ID 가져오기 (실제로는 응답에서 얻어야 함)
      // 예제에서는 간단하게 마지막 주문을 취소하는 것으로 가정
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
  let resp = client.get(&format!("{}/v1/execution?symbol=BTC-KRW", server_base))
    .send()
    .await?;
  
  if resp.status().is_success() {
    let executions: Vec<Value> = resp.json().await?;
    println!("\n총 체결 수: {}", executions.len());
  }
  
  // 작업 정리
  println!("\n프로그램 종료...");
  ob_task.abort();
  exec_task.abort();
  
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
