/**
* filename : matching_engine
* author : HAMA
* date: 2025. 4. 28.
* description: 
**/

use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;
use chrono::Utc;
use crate::models::{Order, OrderMessage, Execution, OrderBook, Side, OrderType, OrderStatus};

pub async fn run(mut order_rx: Receiver<OrderMessage>, exec_tx: Sender<Execution>) {
  let mut book = OrderBook::new();
  
  while let Some(OrderMessage(mut order)) = order_rx.recv().await {
    match order.order_type {
      OrderType::Limit => {
        let mut remaining = order.remain_quantity;
        
        match order.side {
          Side::Buy => {
            // Get matching levels from the opposite book
            let levels = book.sell_book.get_levels_for_matching(order.price);
            for price in levels {
              if remaining == 0 { break; }
              
              if let Some(level) = book.sell_book.limits.get_mut(&price) {
                // Match against orders at this level
                while remaining > 0 && !level.is_empty() {
                  if let Some((matched_order, matched_qty)) = level.match_order(remaining) {
                    // Create execution record
                    let exec = Execution {
                      exec_id: Uuid::new_v4().to_string(),
                      order_id: order.order_id.clone(),
                      symbol: order.symbol.clone(),
                      side: Side::Buy,
                      price,
                      quantity: matched_qty,
                      fee: 0.0,
                      transaction_time: Utc::now()
                    };
                    
                    // Send execution
                    exec_tx.send(exec).await.unwrap();
                    
                    // Update remaining quantity
                    remaining -= matched_qty;
                    order.filled_quantity += matched_qty;
                    
                    // Create execution for the matched order too
                    let counter_exec = Execution {
                      exec_id: Uuid::new_v4().to_string(),
                      order_id: matched_order.order_id.clone(),
                      symbol: matched_order.symbol.clone(),
                      side: Side::Sell,
                      price,
                      quantity: matched_qty,
                      fee: 0.0,
                      transaction_time: Utc::now()
                    };
                    
                    exec_tx.send(counter_exec).await.unwrap();
                  }
                }
                
                // Clean up empty levels
                if level.is_empty() {
                  book.sell_book.limits.remove(&price);
                }
              }
            }
            
            // Update best sell level
            book.sell_book.update_best_level();
          },
          Side::Sell => {
            // Get matching levels from the opposite book
            let levels = book.buy_book.get_levels_for_matching(order.price);
            for price in levels {
              if remaining == 0 { break; }
              
              if let Some(level) = book.buy_book.limits.get_mut(&price) {
                // Match against orders at this level
                while remaining > 0 && !level.is_empty() {
                  if let Some((matched_order, matched_qty)) = level.match_order(remaining) {
                    // Create execution record
                    let exec = Execution {
                      exec_id: Uuid::new_v4().to_string(),
                      order_id: order.order_id.clone(),
                      symbol: order.symbol.clone(),
                      side: Side::Sell,
                      price,
                      quantity: matched_qty,
                      fee: 0.0,
                      transaction_time: Utc::now()
                    };
                    
                    // Send execution
                    exec_tx.send(exec).await.unwrap();
                    
                    // Update remaining quantity
                    remaining -= matched_qty;
                    order.filled_quantity += matched_qty;
                    
                    // Create execution for the matched order too
                    let counter_exec = Execution {
                      exec_id: Uuid::new_v4().to_string(),
                      order_id: matched_order.order_id.clone(),
                      symbol: matched_order.symbol.clone(),
                      side: Side::Buy,
                      price,
                      quantity: matched_qty,
                      fee: 0.0,
                      transaction_time: Utc::now()
                    };
                    
                    exec_tx.send(counter_exec).await.unwrap();
                  }
                }
                
                // Clean up empty levels
                if level.is_empty() {
                  book.buy_book.limits.remove(&price);
                }
              }
            }
            
            // Update best buy level
            book.buy_book.update_best_level();
          }
        }
        
        // Update order status
        order.remain_quantity = remaining;
        if remaining == 0 {
          order.status = OrderStatus::Filled;
        } else if order.filled_quantity > 0 {
          order.status = OrderStatus::PartiallyFilled;
        }
        
        // Insert remaining order to book if not fully filled
        if remaining > 0 {
          book.insert_order(order);
        }
      },
      OrderType::Market => {
        // Handle market orders with similar logic but ignore price constraint
        // Implementation omitted for brevity
      }
    }
  }
}