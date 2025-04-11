use std::thread;
/**
* filename : interagration_test
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use xTraderz::trading_engine::order::Order;
use xTraderz::trading_engine::trading::TradingSystem;

#[test]
fn test_execute_order() {
  let mut trading_system = TradingSystem::new();
  
  // 현재 UNIX 타임스탬프를 기준으로 값을 얻습니다.
  let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
  
  // 즉시 실행되어야 하는 주문 (과거 타임스탬프)
  let order1 = Order {
    order_id: "order1".to_string(),
    timestamp: current_time,
    action: "buy".to_string(),
    quantity: 50,
    price: 100,
  };
  
  // 아직 실행되지 않아야 하는 주문 (미래 타임스탬프)
  let order2 = Order {
    order_id: "order2".to_string(),
    timestamp: current_time + 10,
    action: "sell".to_string(),
    quantity: 75,
    price: 150,
  };
  
  // 두 주문을 지연 주문장에 추가합니다.
  trading_system.add_order(order1.clone());
  trading_system.add_order(order2.clone());
  
  thread::sleep(Duration::from_secs(3));
  // execute_order 호출 시, 주문1은 실행되어 실제 주문장으로 이동해야 합니다.
  trading_system.execute_order();
  
  // 실행된 주문(order_book)에는 order1 만 있어야 합니다.
  let executed_orders = trading_system.order_book.get_orders();
  assert_eq!(executed_orders.len(), 1);
  assert_eq!(executed_orders[0].order_id, order1.order_id);
  
  // 아직 실행되지 않은 주문(delayed_order_book)에는 order2 가 남아 있어야 합니다.
  let pending_orders = trading_system.delayed_order_book.get_orders();
  assert_eq!(pending_orders.len(), 1);
  assert_eq!(pending_orders[0].order_id, order2.order_id);
}
