/**
* filename : serializer
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use serde::{Serialize, Deserialize};
use serde_json::{Value, Error as JsonError};
use chrono::{DateTime, Utc, TimeZone};

use crate::models::{Order, Execution, OrderBook, Book, Side, OrderType, OrderStatus};

/// 주문책 단계 직렬화용 구조체
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceLevelDto {
  pub price: u64,
  pub volume: u64,
  pub order_count: usize,
}

/// 주문책 직렬화용 구조체
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderBookDto {
  pub symbol: String,
  pub timestamp: i64,
  pub bids: Vec<PriceLevelDto>,
  pub asks: Vec<PriceLevelDto>,
}

/// 주문 직렬화용 구조체
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderDto {
  pub order_id: String,
  pub symbol: String,
  pub price: u64,
  pub quantity: u64,
  pub side: String,
  pub order_type: String,
  pub status: String,
  pub filled_quantity: u64,
  pub remain_quantity: u64,
  pub entry_time: String,
}

/// 체결 직렬화용 구조체
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionDto {
  pub exec_id: String,
  pub order_id: String,
  pub symbol: String,
  pub side: String,
  pub price: u64,
  pub quantity: u64,
  pub fee: f64,
  pub transaction_time: String,
}

/// 주문책을 DTO로 변환
pub fn orderbook_to_dto(orderbook: &OrderBook, symbol: &str) -> OrderBookDto {
  let mut bids = Vec::new();
  let mut asks = Vec::new();
  
  // 매수 호가 변환
  for (&price, level) in &orderbook.buy_book.limits {
    bids.push(PriceLevelDto {
      price,
      volume: level.total_volume,
      order_count: level.orders.len(),
    });
  }
  
  // 매도 호가 변환
  for (&price, level) in &orderbook.sell_book.limits {
    asks.push(PriceLevelDto {
      price,
      volume: level.total_volume,
      order_count: level.orders.len(),
    });
  }
  
  // 가격 순서대로 정렬
  bids.sort_by(|a, b| b.price.cmp(&a.price)); // 내림차순 (최고가 먼저)
  asks.sort_by(|a, b| a.price.cmp(&b.price)); // 오름차순 (최저가 먼저)
  
  OrderBookDto {
    symbol: symbol.to_string(),
    timestamp: Utc::now().timestamp_millis(),
    bids,
    asks,
  }
}

/// 주문을 DTO로 변환
pub fn order_to_dto(order: &Order) -> OrderDto {
  OrderDto {
    order_id: order.order_id.clone(),
    symbol: order.symbol.clone(),
    price: order.price,
    quantity: order.quantity,
    side: format!("{:?}", order.side),
    order_type: format!("{:?}", order.order_type),
    status: format!("{:?}", order.status),
    filled_quantity: order.filled_quantity,
    remain_quantity: order.remain_quantity,
    entry_time: order.entry_time.to_rfc3339(),
  }
}

/// 체결을 DTO로 변환
pub fn execution_to_dto(execution: &Execution) -> ExecutionDto {
  ExecutionDto {
    exec_id: execution.exec_id.clone(),
    order_id: execution.order_id.clone(),
    symbol: execution.symbol.clone(),
    side: format!("{:?}", execution.side),
    price: execution.price,
    quantity: execution.quantity,
    fee: execution.fee,
    transaction_time: execution.transaction_time.to_rfc3339(),
  }
}

/// 메시지 직렬화 유틸리티
pub fn serialize<T: Serialize>(value: &T) -> Result<String, JsonError> {
  serde_json::to_string(value)
}

/// 메시지 역직렬화 유틸리티
pub fn deserialize<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, JsonError> {
  serde_json::from_str(json)
}

/// 오더북 델타 변화를 계산하고 직렬화
pub fn calculate_orderbook_delta(
  old_book: &OrderBookDto,
  new_book: &OrderBookDto
) -> Value {
  // 기존 호가와 새 호가의 차이를 계산
  
  // 1. 기존 호가 매핑 (가격 -> 인덱스)
  let mut old_bids_map = std::collections::HashMap::new();
  for (i, bid) in old_book.bids.iter().enumerate() {
    old_bids_map.insert(bid.price, i);
  }
  
  let mut old_asks_map = std::collections::HashMap::new();
  for (i, ask) in old_book.asks.iter().enumerate() {
    old_asks_map.insert(ask.price, i);
  }
  
  // 2. 변경된 호가 계산
  let mut added_bids = Vec::new();
  let mut updated_bids = Vec::new();
  let mut removed_bids = Vec::new();
  
  for bid in &new_book.bids {
    if !old_bids_map.contains_key(&bid.price) {
      added_bids.push(bid.clone());
    } else {
      let old_idx = old_bids_map[&bid.price];
      if old_book.bids[old_idx].volume != bid.volume {
        updated_bids.push(bid.clone());
      }
      old_bids_map.remove(&bid.price);
    }
  }
  
  // 제거된 호가
  for &price in old_bids_map.keys() {
    removed_bids.push(price);
  }
  
  // 매도 호가에 대해서도 동일한 작업 수행
  let mut added_asks = Vec::new();
  let mut updated_asks = Vec::new();
  let mut removed_asks = Vec::new();
  
  for ask in &new_book.asks {
    if !old_asks_map.contains_key(&ask.price) {
      added_asks.push(ask.clone());
    } else {
      let old_idx = old_asks_map[&ask.price];
      if old_book.asks[old_idx].volume != ask.volume {
        updated_asks.push(ask.clone());
      }
      old_asks_map.remove(&ask.price);
    }
  }
  
  // 제거된 호가
  for &price in old_asks_map.keys() {
    removed_asks.push(price);
  }
  
  // 델타 JSON 생성
  serde_json::json!({
        "type": "orderbook_delta",
        "symbol": new_book.symbol,
        "timestamp": new_book.timestamp,
        "bids": {
            "added": added_bids,
            "updated": updated_bids,
            "removed": removed_bids
        },
        "asks": {
            "added": added_asks,
            "updated": updated_asks,
            "removed": removed_asks
        }
    })
}

/// WebSocket 메시지 생성 유틸리티
pub fn create_websocket_message(
  message_type: &str,
  data: &impl Serialize
) -> Result<String, JsonError> {
  let payload = match serde_json::to_value(data)? {
    Value::Object(obj) => obj,
    _ => return Err(serde_json::Error::custom("Expected object value"))
  };
  
  let message = serde_json::json!({
        "type": message_type,
        "timestamp": Utc::now().timestamp_millis(),
        "data": payload
    });
  
  serde_json::to_string(&message)
}