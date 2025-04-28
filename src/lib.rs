/**
* filename : lib
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

pub mod models;
pub mod order_book;
pub mod matching_engine;
pub mod sequencer;
pub mod order_manager;

#[cfg(test)]
mod tests {
  use super::*;
  use uuid::Uuid;
  use chrono::Utc;
  use crate::models::{Order, OrderSide, OrderType, OrderStatus, TimeInForce};
  use crate::matching_engine::process_order;
  
  #[test]
  fn test_price_level_basic() {
    let mut pl = order_book::PriceLevel::new(100);
    let order = Order {
      order_id: Uuid::new_v4(),
      product_id: "XYZ".to_string(),
      price: 100,
      original_quantity: 5,
      remaining_quantity: 5,
      filled_quantity: 0,
      side: OrderSide::Buy,
      status: OrderStatus::New,
      order_type: OrderType::Limit,
      time_in_force: TimeInForce::Gtc,
      symbol: "XYZ".to_string(),
      user_id: "user1".to_string(),
      client_order_id: None,
      broker: None,
      account_id: None,
      entry_time: Utc::now(),
      transaction_time: Utc::now(),
    };
    pl.add_order(order.clone());
    assert_eq!(pl.total_volume, order.remaining_quantity);
    assert_eq!(pl.orders.front().unwrap().order_id, order.order_id);
  }
  
  #[test]
  fn test_order_book_insert_and_remove() {
    let mut book = order_book::OrderBook::new();
    let order = Order {
      order_id: Uuid::new_v4(),
      product_id: "ABC".to_string(),
      price: 200,
      original_quantity: 10,
      remaining_quantity: 10,
      filled_quantity: 0,
      side: OrderSide::Sell,
      status: OrderStatus::New,
      order_type: OrderType::Limit,
      time_in_force: TimeInForce::Gtc,
      symbol: "ABC".to_string(),
      user_id: "user2".to_string(),
      client_order_id: None,
      broker: None,
      account_id: None,
      entry_time: Utc::now(),
      transaction_time: Utc::now(),
    };
    book.insert_order(order.clone());
    assert!(book.order_map.contains_key(&order.order_id));
    let removed = book.remove_order(&order.order_id).expect("order should be removed");
    assert_eq!(removed.order_id, order.order_id);
    assert!(!book.order_map.contains_key(&order.order_id));
  }
  
  #[tokio::test]
  async fn test_matching_engine_limit_buy() {
    let mut book = order_book::OrderBook::new();
    // 주문장이 있는 매도 호가 추가
    let ask = Order {
      order_id: Uuid::new_v4(),
      product_id: "TST".to_string(),
      price: 100,
      original_quantity: 10,
      remaining_quantity: 10,
      filled_quantity: 0,
      side: OrderSide::Sell,
      status: OrderStatus::New,
      order_type: OrderType::Limit,
      time_in_force: TimeInForce::Gtc,
      symbol: "TST".to_string(),
      user_id: "seller".to_string(),
      client_order_id: None,
      broker: None,
      account_id: None,
      entry_time: Utc::now(),
      transaction_time: Utc::now(),
    };
    book.insert_order(ask.clone());
    // 매수 주문 생성
    let bid = Order {
      order_id: Uuid::new_v4(),
      product_id: "TST".to_string(),
      price: 110,
      original_quantity: 5,
      remaining_quantity: 5,
      filled_quantity: 0,
      side: OrderSide::Buy,
      status: OrderStatus::New,
      order_type: OrderType::Limit,
      time_in_force: TimeInForce::Gtc,
      symbol: "TST".to_string(),
      user_id: "buyer".to_string(),
      client_order_id: None,
      broker: None,
      account_id: None,
      entry_time: Utc::now(),
      transaction_time: Utc::now(),
    };
    let executions = process_order(&mut book, bid.clone()).await;
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].quantity, 5);
    assert_eq!(executions[0].price, 100);
  }
}
