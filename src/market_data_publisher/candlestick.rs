/**
* filename : candlestick
* author : HAMA
* date: 2025. 5. 1.
* description: 
**/

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Duration, Utc};
use crate::models::Execution;
use super::models::{Candle, CandleInterval, CircularBuffer};

// 봉차트 관리자
pub struct CandlestickManager {
  // 심볼 -> 간격 -> 캔들 버퍼
  candles: Arc<Mutex<HashMap<String, HashMap<CandleInterval, CircularBuffer<Candle>>>>>,
  // 심볼 -> 간격 -> 현재 진행 중인 캔들
  current_candles: Arc<Mutex<HashMap<String, HashMap<CandleInterval, Candle>>>>,
}

impl CandlestickManager {
  // 새 캔들스틱 관리자 생성
  pub fn new() -> Self {
    CandlestickManager {
      candles: Arc::new(Mutex::new(HashMap::new())),
      current_candles: Arc::new(Mutex::new(HashMap::new())),
    }
  }
  
  // 체결 처리
  pub fn process_execution(&self, execution: &Execution) {
    let timestamp = execution.transaction_time;
    let symbol = &execution.symbol;
    let price = execution.price;
    let volume = execution.quantity;
    
    // 모든 시간 간격에 대해 처리
    for interval in self.get_all_intervals() {
      self.update_candle(symbol, timestamp, price, volume, interval);
    }
  }
  
  // 지원하는 모든 시간 간격
  fn get_all_intervals(&self) -> Vec<CandleInterval> {
    vec![
      CandleInterval::Minute1,
      CandleInterval::Minute5,
      CandleInterval::Minute15,
      CandleInterval::Minute30,
      CandleInterval::Hour1,
      CandleInterval::Hour4,
      CandleInterval::Day1,
      CandleInterval::Week1,
    ]
  }
  
  // 캔들 업데이트
  fn update_candle(&self, symbol: &str, timestamp: DateTime<Utc>, price: u64, volume: u64, interval: CandleInterval) {
    let interval_seconds = interval.to_seconds();
    
    // 현재 캔들의 시작 시간 계산
    let start_time = self.calculate_candle_start_time(timestamp, interval_seconds);
    let end_time = start_time + Duration::seconds(interval_seconds);
    
    let mut current_candles = self.current_candles.lock().unwrap();
    
    // 심볼에 대한 현재 캔들 맵 가져오기
    let symbol_candles = current_candles
      .entry(symbol.to_string())
      .or_insert_with(HashMap::new);
    
    // 현재 진행 중인 캔들 가져오기
    if let Some(candle) = symbol_candles.get_mut(&interval) {
      // 캔들이 이미 있고 같은 시간 간격에 있는 경우 업데이트
      if candle.open_time == start_time {
        // 최고가, 최저가 업데이트
        candle.high = candle.high.max(price);
        candle.low = candle.low.min(price);
        // 종가 업데이트
        candle.close = price;
        // 거래량 업데이트
        candle.volume += volume;
        // 거래 횟수 증가
        candle.trade_count += 1;
      } else {
        // 이전 캔들 완료
        let completed_candle = candle.clone();
        self.store_completed_candle(completed_candle);
        
        // 새 캔들 시작
        *candle = self.create_new_candle(symbol, start_time, end_time, interval, price, volume);
      }
    } else {
      // 처음 생성하는 캔들
      let new_candle = self.create_new_candle(symbol, start_time, end_time, interval, price, volume);
      symbol_candles.insert(interval, new_candle);
    }
  }
  
  // 캔들 시작 시간 계산
  fn calculate_candle_start_time(&self, timestamp: DateTime<Utc>, interval_seconds: i64) -> DateTime<Utc> {
    let seconds_since_epoch = timestamp.timestamp();
    let candle_start_seconds = (seconds_since_epoch / interval_seconds) * interval_seconds;
    Utc.timestamp(candle_start_seconds, 0)
  }
  
  // 새 캔들 생성
  fn create_new_candle(
    &self,
    symbol: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval: CandleInterval,
    price: u64,
    volume: u64
  ) -> Candle {
    Candle {
      symbol: symbol.to_string(),
      open_time: start_time,
      close_time: end_time,
      interval,
      open: price,
      high: price,
      low: price,
      close: price,
      volume,
      trade_count: 1,
    }
  }
  
  // 완료된 캔들 저장
  fn store_completed_candle(&self, candle: Candle) {
    let mut candles = self.candles.lock().unwrap();
    
    // 심볼에 대한 캔들 맵 가져오기
    let symbol_candles = candles
      .entry(candle.symbol.clone())
      .or_insert_with(HashMap::new);
    
    // 간격에 대한 캔들 버퍼 가져오기
    let candle_buffer = symbol_candles
      .entry(candle.interval.clone())
      .or_insert_with(|| CircularBuffer::new(self.get_buffer_capacity(&candle.interval)));
    
    // 캔들 저장
    candle_buffer.push(candle);
  }
  
  // 간격에 따른 버퍼 용량 결정
  fn get_buffer_capacity(&self, interval: &CandleInterval) -> usize {
    match interval {
      CandleInterval::Minute1 => 1440,  // 하루 (24시간 * 60분)
      CandleInterval::Minute5 => 1152,  // 4일 (24시간 * 12개/시간 * 4일)
      CandleInterval::Minute15 => 960,  // 10일 (24시간 * 4개/시간 * 10일)
      CandleInterval::Minute30 => 1008, // 3주 (24시간 * 2개/시간 * 21일)
      CandleInterval::Hour1 => 720,     // 30일 (24시간 * 30일)
      CandleInterval::Hour4 => 720,     // 120일 (6개/일 * 120일)
      CandleInterval::Day1 => 365,      // 1년
      CandleInterval::Week1 => 156,     // 3년 (52주 * 3년)
    }
  }
  
  // 캔들 데이터 가져오기
  pub fn get_candles(&self, symbol: &str, interval: CandleInterval, limit: Option<usize>) -> Vec<Candle> {
    let candles = self.candles.lock().unwrap();
    
    // 심볼에 대한 캔들 맵 확인
    if let Some(symbol_candles) = candles.get(symbol) {
      // 간격에 대한 캔들 버퍼 확인
      if let Some(candle_buffer) = symbol_candles.get(&interval) {
        // 제한이 있으면 최근 N개만 반환, 없으면 전체 반환
        if let Some(count) = limit {
          candle_buffer.get_recent(count)
        } else {
          candle_buffer.get_all()
        }
      } else {
        Vec::new()
      }
    } else {
      Vec::new()
    }
  }
  
  // 현재 진행 중인 캔들 가져오기
  pub fn get_current_candle(&self, symbol: &str, interval: CandleInterval) -> Option<Candle> {
    let current_candles = self.current_candles.lock().unwrap();
    
    current_candles.get(symbol)
      .and_then(|symbol_candles| symbol_candles.get(&interval))
      .cloned()
  }
}