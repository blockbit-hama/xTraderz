/**
* filename : server
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::{self, Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::models::{OrderBook};
use crate::util::serializer;
use super::client_handler::{ClientMessage, ClientUpdate};

// 릴레이 시뮬레이션 서버 - 오더북 상태를 클라이언트에 전달
pub struct RelayServer {
  // 각 심볼별 오더북 상태를 저장
  orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
  // 클라이언트 업데이트를 위한 송신기
  client_tx: Sender<ClientUpdate>,
  // 구독 상태 관리 (심볼 -> 구독 여부)
  subscriptions: Arc<Mutex<HashMap<String, AtomicBool>>>,
}

impl RelayServer {
  pub fn new() -> (Self, Receiver<ClientUpdate>) {
    let (client_tx, client_rx) = channel(100);
    
    (RelayServer {
      orderbooks: Arc::new(Mutex::new(HashMap::new())),
      client_tx,
      subscriptions: Arc::new(Mutex::new(HashMap::new())),
    }, client_rx)
  }
  
  // 오더북 상태 업데이트
  pub fn update_orderbook(&self, symbol: &str, orderbook: OrderBook) {
    let mut books = self.orderbooks.lock().unwrap();
    books.insert(symbol.to_string(), orderbook);
    
    // 해당 심볼이 구독 중인지 확인
    let subs = self.subscriptions.lock().unwrap();
    if let Some(is_subscribed) = subs.get(symbol) {
      if is_subscribed.load(Ordering::Relaxed) {
        // 구독 중이면 클라이언트에 업데이트 전송
        let _ = self.send_orderbook_update(symbol);
      }
    }
  }
  
  // 클라이언트 메시지 처리
  async fn process_client_message(&self, message: ClientMessage) {
    match message {
      ClientMessage::SubscribeOrderBook(symbol) => {
        println!("오더북 구독: {}", symbol);
        
        // 구독 상태 업데이트
        let mut subs = self.subscriptions.lock().unwrap();
        let subscription = subs.entry(symbol.clone()).or_insert_with(|| AtomicBool::new(false));
        subscription.store(true, Ordering::Relaxed);
        
        // 초기 스냅샷 전송
        drop(subs); // 락 해제
        self.send_orderbook_snapshot(&symbol);
      },
      ClientMessage::UnsubscribeOrderBook(symbol) => {
        println!("오더북 구독 해제: {}", symbol);
        
        // 구독 상태 업데이트
        let mut subs = self.subscriptions.lock().unwrap();
        if let Some(subscription) = subs.get(&symbol) {
          subscription.store(false, Ordering::Relaxed);
        }
      },
      ClientMessage::RequestOrderBookSnapshot(symbol) => {
        println!("오더북 스냅샷 요청: {}", symbol);
        
        // 스냅샷 전송
        self.send_orderbook_snapshot(&symbol);
      }
    }
  }
  
  // 오더북 스냅샷 전송
  fn send_orderbook_snapshot(&self, symbol: &str) {
    let books = self.orderbooks.lock().unwrap();
    if let Some(orderbook) = books.get(symbol) {
      // 오더북 DTO로 변환
      let dto = serializer::orderbook_to_dto(orderbook, symbol);
      
      // 클라이언트에 전송
      let _ = self.client_tx.try_send(ClientUpdate::OrderBookSnapshot {
        symbol: symbol.clone(),
        data: dto,
      });
    }
  }
  
  // 오더북 업데이트 전송
  fn send_orderbook_update(&self, symbol: &str) -> bool {
    let books = self.orderbooks.lock().unwrap();
    if let Some(orderbook) = books.get(symbol) {
      // 오더북 DTO로 변환
      let dto = serializer::orderbook_to_dto(orderbook, symbol);
      
      // 클라이언트에 전송
      match self.client_tx.try_send(ClientUpdate::OrderBookUpdate {
        symbol: symbol.clone(),
        data: dto,
      }) {
        Ok(_) => return true,
        Err(_) => return false,
      }
    }
    false
  }
  
  // 주기적 업데이트 전송
  async fn send_periodic_updates(&self) {
    let subs = self.subscriptions.lock().unwrap();
    
    for (symbol, is_subscribed) in subs.iter() {
      if is_subscribed.load(Ordering::Relaxed) {
        // 락 해제 후 업데이트 전송
        drop(subs.clone());
        self.send_orderbook_update(symbol);
        break; // 락을 풀었으니 반복문을 종료하고 다시 시작
      }
    }
  }
  
  // 서버 실행
  pub async fn run(&self, mut client_msg_rx: Receiver<ClientMessage>) {
    let mut update_interval = time::interval(Duration::from_millis(200));
    
    println!("릴레이 서버 시작...");
    
    loop {
      tokio::select! {
                Some(client_msg) = client_msg_rx.recv() => {
                    self.process_client_message(client_msg).await;
                }
                _ = update_interval.tick() => {
                    self.send_periodic_updates().await;
                }
            }
    }
  }
}