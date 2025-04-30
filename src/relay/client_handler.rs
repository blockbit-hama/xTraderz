/**
* filename : client_handler
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::{Receiver, Sender};
use crate::util::serializer;

// 클라이언트로부터 받는 메시지
#[derive(Debug, Clone)]
pub enum ClientMessage {
  SubscribeOrderBook(String),     // 심볼 구독
  UnsubscribeOrderBook(String),   // 구독 해제
  RequestOrderBookSnapshot(String), // 오더북 스냅샷 요청
}

// 클라이언트에게 보내는 업데이트
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ClientUpdate {
  #[serde(rename = "orderbook_snapshot")]
  OrderBookSnapshot {
    symbol: String,
    #[serde(flatten)]
    data: serializer::OrderBookDto,
  },
  
  #[serde(rename = "orderbook_update")]
  OrderBookUpdate {
    symbol: String,
    #[serde(flatten)]
    data: serializer::OrderBookDto,
  },
}

// 클라이언트 핸들러
pub struct ClientHandler {
  client_tx: Sender<ClientUpdate>,
  server_tx: Sender<ClientMessage>,
}

impl ClientHandler {
  pub fn new(
    client_tx: Sender<ClientUpdate>,
    server_tx: Sender<ClientMessage>,
  ) -> Self {
    ClientHandler {
      client_tx,
      server_tx,
    }
  }
  
  // 클라이언트에서 받은 메시지 처리
  pub async fn handle_message(&self, message: &str) -> Result<(), String> {
    // 메시지 파싱 및 처리
    if message.starts_with("subscribe:") {
      let symbol = message.strip_prefix("subscribe:").unwrap().to_string();
      self.server_tx.send(ClientMessage::SubscribeOrderBook(symbol))
        .await
        .map_err(|_| "Failed to send message to server".to_string())?;
    } else if message.starts_with("unsubscribe:") {
      let symbol = message.strip_prefix("unsubscribe:").unwrap().to_string();
      self.server_tx.send(ClientMessage::UnsubscribeOrderBook(symbol))
        .await
        .map_err(|_| "Failed to send message to server".to_string())?;
    } else if message.starts_with("snapshot:") {
      let symbol = message.strip_prefix("snapshot:").unwrap().to_string();
      self.server_tx.send(ClientMessage::RequestOrderBookSnapshot(symbol))
        .await
        .map_err(|_| "Failed to send message to server".to_string())?;
    } else {
      return Err(format!("Unknown command: {}", message));
    }
    
    Ok(())
  }
  
  // 서버에서 받은 업데이트 전달
  pub async fn run_updates(&self, mut rx: Receiver<ClientUpdate>) {
    while let Some(update) = rx.recv().await {
      // 클라이언트에 업데이트 전달
      if let Ok(json) = serde_json::to_string(&update) {
        println!("클라이언트에 전송: {}", json);
        // 실제 구현에서는 WebSocket 등을 통해 전송
        // socket.send(json).await...
      }
    }
  }
}