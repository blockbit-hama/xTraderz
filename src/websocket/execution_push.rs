/**
* filename : execution_push
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::sync::{Arc, Mutex};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::{Receiver, Sender};
use warp::ws::{Message, WebSocket};
use warp::Filter;
use serde_json::json;

use crate::models::Execution;
use crate::util::serializer;

// WebSocket 연결을 관리하는 구조체
pub struct ExecutionPushManager {
  connections: Arc<Mutex<Vec<Sender<Message>>>>,
}

impl ExecutionPushManager {
  pub fn new() -> Self {
    ExecutionPushManager {
      connections: Arc::new(Mutex::new(Vec::new())),
    }
  }
  
  // 새 연결 추가
  pub fn add_connection(&self, tx: Sender<Message>) {
    let mut connections = self.connections.lock().unwrap();
    connections.push(tx);
  }
  
  // 연결 제거
  pub fn remove_connection(&self, tx: &Sender<Message>) {
    let mut connections = self.connections.lock().unwrap();
    if let Some(pos) = connections.iter().position(|x| x.same_channel(tx)) {
      connections.remove(pos);
    }
  }
  
  // 모든 연결에 체결 정보 전송
  pub async fn broadcast_execution(&self, execution: &Execution) {
    let connections = self.connections.lock().unwrap().clone();
    
    // 체결을 DTO로 변환하고 직렬화
    let exec_dto = serializer::execution_to_dto(execution);
    let exec_json = match serde_json::to_string(&exec_dto) {
      Ok(json) => json,
      Err(_) => return, // 직렬화 실패 시 종료
    };
    
    for tx in &connections {
      if let Err(_) = tx.send(Message::text(exec_json.clone())).await {
        // 에러가 발생하면 나중에 연결을 제거하기 위해 표시
        // 실제 구현에서는 오류 처리 로직을 더 견고하게 구현해야 함
      }
    }
  }
}

// WebSocket 요청 처리 라우트
pub fn ws_execution_route(
  manager: Arc<ExecutionPushManager>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  warp::path!("ws" / "executions")
    .and(warp::ws())
    .map(move |ws: warp::ws::Ws| {
      let manager = manager.clone();
      ws.on_upgrade(move |socket| handle_execution_connection(socket, manager))
    })
}

// 새 WebSocket 연결 처리
async fn handle_execution_connection(ws: WebSocket, manager: Arc<ExecutionPushManager>) {
  let (ws_tx, mut ws_rx) = ws.split();
  
  // 토키오 채널 생성 (WebSocket 메시지 전송용)
  let (tx, rx) = tokio::sync::mpsc::channel::<Message>(100);
  let manager_clone = manager.clone();
  
  // 메시지 수신 작업
  tokio::task::spawn(rx.forward(ws_tx).map(|result| {
    if let Err(e) = result {
      eprintln!("WebSocket send error: {}", e);
    }
  }));
  
  // 연결 등록
  manager.add_connection(tx.clone());
  
  // 클라이언트로부터의 메시지 처리 (여기서는 단순히 연결 상태 확인)
  while let Some(result) = ws_rx.next().await {
    match result {
      Ok(_) => {
        // 메시지 처리 (필요시)
      }
      Err(_) => {
        manager_clone.remove_connection(&tx);
        break;
      }
    }
  }
  
  // 연결 종료 시 정리
  manager_clone.remove_connection(&tx);
}

// 체결 수신 및 WebSocket 브로드캐스트 작업
pub async fn run_execution_broadcaster(
  mut exec_rx: Receiver<Execution>,
  manager: Arc<ExecutionPushManager>
) {
  while let Some(execution) = exec_rx.recv().await {
    manager.broadcast_execution(&execution).await;
  }
}