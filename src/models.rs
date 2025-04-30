/**
* filename : models
* author : HAMA
* date: 2025. 4. 28.
* description: 
**/

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::cmp::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Side { Buy, Sell }

impl Side {
  pub fn compare(&self, a: &u64, b: &u64) -> Ordering {
    match self {
      Side::Buy => b.cmp(a),  // Buy: descending (highest price first)
      Side::Sell => a.cmp(b), // Sell: ascending (lowest price first)
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType { Limit, Market }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderStatus { New, PartiallyFilled, Filled, Cancelled }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
  pub order_id: String,
  pub symbol: String,
  pub price: u64,
  pub quantity: u64,
  pub side: Side,
  pub order_type: OrderType,
  pub status: OrderStatus,
  pub filled_quantity: u64,
  pub remain_quantity: u64,
  pub entry_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Execution {
  pub exec_id: String,
  pub order_id: String,
  pub symbol: String,
  pub side: Side,
  pub price: u64,
  pub quantity: u64,
  pub fee: f64,
  pub transaction_time: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct OrderMessage(pub Order);

#[derive(Clone, Debug)]
pub struct OrderReference {
  pub price: u64,
  pub position: usize, // Position in the queue for O(1) access
}

#[derive(Clone, Debug)]
pub struct PriceLevel {
  pub price: u64,
  pub total_volume: u64,
  pub orders: VecDeque<Order>,
}

impl PriceLevel {
  pub fn new(price: u64) -> Self {
    PriceLevel {
      price,
      total_volume: 0,
      orders: VecDeque::new()
    }
  }
  
  pub fn add_order(&mut self, order: Order) -> usize {
    self.total_volume += order.remain_quantity;
    let position = self.orders.len();
    self.orders.push_back(order);
    position
  }
  
  pub fn match_order(&mut self, qty: u64) -> Option<(Order, u64)> {
    if let Some(mut front) = self.orders.front_mut() {
      let matched = std::cmp::min(front.remain_quantity, qty);
      front.remain_quantity -= matched;
      front.filled_quantity += matched;
      self.total_volume -= matched;
      
      if front.remain_quantity == 0 {
        front.status = OrderStatus::Filled;
        return Some((self.orders.pop_front().unwrap(), matched));
      } else {
        front.status = OrderStatus::PartiallyFilled;
        return Some((front.clone(), matched));
      }
    }
    None
  }
  
  pub fn cancel_order_at_position(&mut self, position: usize) -> Option<Order> {
    if position < self.orders.len() {
      let order = self.orders.remove(position).unwrap();
      self.total_volume -= order.remain_quantity;
      Some(order)
    } else {
      None
    }
  }
  
  pub fn is_empty(&self) -> bool {
    self.orders.is_empty()
  }
}

#[derive(Clone, Debug)]
pub struct Book {
  pub side: Side,
  pub limits: BTreeMap<u64, PriceLevel>,
  pub best_level: Option<u64>,
}

impl Book {
  pub fn new(side: Side) -> Self {
    Book {
      side,
      limits: BTreeMap::new(),
      best_level: None
    }
  }
  
  pub fn add_order(&mut self, order: Order) -> OrderReference {
    let price = order.price;
    let level = self.limits.entry(price).or_insert_with(|| PriceLevel::new(price));
    let position = level.add_order(order);
    
    // Update best level
    match self.side {
      Side::Buy => {
        if self.best_level.is_none() || price > self.best_level.unwrap() {
          self.best_level = Some(price);
        }
      },
      Side::Sell => {
        if self.best_level.is_none() || price < self.best_level.unwrap() {
          self.best_level = Some(price);
        }
      }
    }
    
    OrderReference { price, position }
  }
  
  pub fn get_best_level(&self) -> Option<&PriceLevel> {
    self.best_level.and_then(|price| self.limits.get(&price))
  }
  
  pub fn get_best_level_mut(&mut self) -> Option<&mut PriceLevel> {
    let price = self.best_level?;
    self.limits.get_mut(&price)
  }
  
  pub fn update_best_level(&mut self) {
    self.best_level = match self.side {
      Side::Buy => self.limits.keys().next_back().cloned(),
      Side::Sell => self.limits.keys().next().cloned(),
    };
  }
  
  pub fn get_levels_for_matching(&self, price_point: u64) -> Vec<u64> {
    let mut result = Vec::new();
    
    match self.side {
      Side::Buy => {
        // For buy book, iterate in reverse to get highest prices first
        for &level_price in self.limits.keys().rev() {
          if level_price < price_point {
            break;
          }
          result.push(level_price);
        }
      },
      Side::Sell => {
        // For sell book, iterate normally to get lowest prices first
        for &level_price in self.limits.keys() {
          if level_price > price_point {
            break;
          }
          result.push(level_price);
        }
      }
    }
    
    result
  }
}

#[derive(Clone, Debug)]
pub struct OrderBook {
  pub buy_book: Book,
  pub sell_book: Book,
  pub order_map: HashMap<String, OrderReference>,
}

impl OrderBook {
  pub fn new() -> Self {
    OrderBook {
      buy_book: Book::new(Side::Buy),
      sell_book: Book::new(Side::Sell),
      order_map: HashMap::new()
    }
  }
  
  pub fn insert_order(&mut self, order: Order) {
    let book = match order.side {
      Side::Buy => &mut self.buy_book,
      Side::Sell => &mut self.sell_book,
    };
    
    let order_ref = book.add_order(order.clone());
    self.order_map.insert(order.order_id.clone(), order_ref);
  }
  
  pub fn cancel_order(&mut self, order_id: &str) -> Option<Order> {
    if let Some(order_ref) = self.order_map.remove(order_id) {
      let book = match self.get_book_by_price(order_ref.price) {
        Some(b) => b,
        None => return None,
      };
      
      if let Some(level) = book.limits.get_mut(&order_ref.price) {
        let cancelled_order = level.cancel_order_at_position(order_ref.position);
        
        // Clean up empty levels and update best price
        if level.is_empty() {
          book.limits.remove(&order_ref.price);
          book.update_best_level();
        }
        
        return cancelled_order;
      }
    }
    None
  }
  
  fn get_book_by_price(&mut self, price: u64) -> Option<&mut Book> {
    // Try to find the price in either book
    if self.buy_book.limits.contains_key(&price) {
      Some(&mut self.buy_book)
    } else if self.sell_book.limits.contains_key(&price) {
      Some(&mut self.sell_book)
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Utc;
  
  #[test]
  fn test_limit_level_add_and_match() {
    let mut pl = PriceLevel::new(100);
    let order = Order {
      order_id: "o1".into(),
      symbol: "SYM".into(),
      price: 100,
      quantity: 10,
      side: Side::Buy,
      order_type: OrderType::Limit,
      status: OrderStatus::New,
      filled_quantity: 0,
      remain_quantity: 10,
      entry_time: Utc::now()
    };
    
    pl.add_order(order.clone());
    assert_eq!(pl.total_volume, 10);
    
    let (matched, qty) = pl.match_order(5).unwrap();
    assert_eq!(matched.remain_quantity, 5);
    assert_eq!(matched.filled_quantity, 5);
    assert_eq!(qty, 5);
    assert_eq!(pl.total_volume, 5);
  }
  
  #[test]
  fn test_book_best_level() {
    let mut book = Book::new(Side::Buy);
    
    // Add orders at different prices
    let order1 = Order {
      order_id: "o1".into(),
      symbol: "SYM".into(),
      price: 100,
      quantity: 5,
      side: Side::Buy,
      order_type: OrderType::Limit,
      status: OrderStatus::New,
      filled_quantity: 0,
      remain_quantity: 5,
      entry_time: Utc::now()
    };
    
    let order2 = Order {
      order_id: "o2".into(),
      symbol: "SYM".into(),
      price: 105,
      quantity: 3,
      side: Side::Buy,
      order_type: OrderType::Limit,
      status: OrderStatus::New,
      filled_quantity: 0,
      remain_quantity: 3,
      entry_time: Utc::now()
    };
    
    book.add_order(order1);
    assert_eq!(book.best_level, Some(100));
    
    book.add_order(order2);
    assert_eq!(book.best_level, Some(105)); // For buy book, highest price is best
    
    // Verify level retrieval
    let best = book.get_best_level().unwrap();
    assert_eq!(best.price, 105);
    assert_eq!(best.total_volume, 3);
  }
  
  #[test]
  fn test_order_book_cancel() {
    let mut ob = OrderBook::new();
    
    let order = Order {
      order_id: "o2".into(),
      symbol: "SYM".into(),
      price: 50,
      quantity: 5,
      side: Side::Buy,
      order_type: OrderType::Limit,
      status: OrderStatus::New,
      filled_quantity: 0,
      remain_quantity: 5,
      entry_time: Utc::now()
    };
    
    ob.insert_order(order.clone());
    assert!(ob.buy_book.limits.contains_key(&50));
    
    let cancelled = ob.cancel_order("o2").unwrap();
    assert_eq!(cancelled.order_id, "o2");
    assert!(!ob.buy_book.limits.contains_key(&50));
    assert!(ob.order_map.get("o2").is_none());
  }
}