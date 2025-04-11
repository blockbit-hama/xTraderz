/**
* filename : trading
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

use crate::trading_engine::order::Order;
use crate::trading_engine::order_book::OrderBook;
use std::time::{SystemTime, UNIX_EPOCH};


#[derive(Debug)]
pub struct TradingSystem {
  pub delayed_order_book: OrderBook,
  pub order_book: OrderBook,
}

impl TradingSystem {
  pub fn new() -> Self {
    TradingSystem {
      delayed_order_book: OrderBook::new(),
      order_book: OrderBook::new()
    }
  }
  
  /// 주문은 즉시 실행하지 않고, 지연 주문장에 추가합니다.
  pub fn add_order(&mut self, order: Order) {
    self.delayed_order_book.add_order(order);
  }
  
  /// 지연 주문장에서 실행 예정인 주문들을 찾아 실행시키고,
  /// 실행된 주문은 지연 주문장에서 제거합니다.
  pub fn execute_order(&mut self) {
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    // delayed_order_book에 저장된 모든 pending order를 하나의 Vec로 수집합니다.
    let pending_orders = self.delayed_order_book.get_orders();
    
    // 현재 시각(current_time) 보다 timestamp가 작거나 같은 주문은 실행 대상으로, 나머지는 보류 대상으로 partition 합니다.
    let (to_execute, remaining): (Vec<_>, Vec<_>) = pending_orders.into_iter()
      .partition(|order| order.timestamp <= current_time);
    
    // 실행할 주문들은 order_book에 추가합니다.
    for order in to_execute {
      self.order_book.add_order(order);
      println!("Order executed");
    }
    
    // 보류 대상 주문들을 delayed_order_book에 재설정합니다.
    self.delayed_order_book.buy_orders.orders.clear();
    self.delayed_order_book.sell_orders.orders.clear();
    
    for order in remaining {
      if order.action == "buy" {
        self.delayed_order_book.buy_orders.add_order(order);
      } else {
        self.delayed_order_book.sell_orders.add_order(order);
      }
    }
  }
}
