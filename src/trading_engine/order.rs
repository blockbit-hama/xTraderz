/**
* filename : order
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

use std::collections::HashMap;
use serde::Deserialize;

#[derive(Clone, Debug,Deserialize)]
pub struct Order {
  pub order_id: String,
  pub timestamp: u64, // 실행 예정 시각 (초 단위)
  pub action: String, // "buy" 또는 "sell"
  pub quantity: u64,
  pub price: u64,
}

#[derive(Clone, Debug)]
pub struct OrderQueue {
  pub orders: HashMap<String, Order>,
}

impl OrderQueue {
  pub fn new() -> Self {
    OrderQueue { orders: HashMap::new() }
  }
  
  pub fn add_order(&mut self, order: Order) {
    self.orders.insert(order.order_id.clone(), order);
  }
  
  pub fn get_orders(&self) -> Vec<Order> {
    self.orders.values().cloned().collect()
  }
}
