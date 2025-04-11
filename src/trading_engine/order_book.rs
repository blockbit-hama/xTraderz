/**
* filename : order_book
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

use crate::trading_engine::order::{Order, OrderQueue};

#[derive(Clone, Debug)]
pub struct OrderBook {
  pub buy_orders: OrderQueue,
  pub sell_orders: OrderQueue,
}

impl OrderBook {
  pub fn new() -> Self {
    OrderBook {
      buy_orders: OrderQueue::new(),
      sell_orders: OrderQueue::new()
    }
  }
  
  pub fn add_order(&mut self, order: Order) {
    if order.action == "buy" {
      self.buy_orders.add_order(order);
    } else {
      self.sell_orders.add_order(order);
    }
  }
  
  pub fn get_orders(&self) -> Vec<Order> {
    let mut orders = self.buy_orders.get_orders();
    orders.extend(self.sell_orders.get_orders());
    orders
  }
  
}
