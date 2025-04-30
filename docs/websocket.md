# WebSocket 프로토콜 문서

이 문서는 주문 매칭 엔진의 WebSocket 프로토콜에 대한 상세 정보를 제공합니다.

## 개요

주문 매칭 엔진은 두 개의 독립적인 WebSocket 서비스를 제공합니다:

1. **체결 스트림**: 실시간 체결 정보 제공 (모든 체결 브로드캐스트)
2. **오더북 스트림**: 실시간 오더북 상태 및 변경사항 제공 (심볼별 구독)

이 두 스트림은 **주문 처리 파이프라인과 완전히 독립적**으로 작동합니다. 이는 UI 업데이트 트래픽이 핵심 거래 기능에 영향을 주지 않도록 하기 위한 설계입니다.

## 연결 정보

- WebSocket 기본 URL: `ws://127.0.0.1:3030`
- 프로토콜: WebSocket (RFC 6455)
- 인코딩: UTF-8
- 메시지 형식: JSON

## 체결 스트림

### 연결 URL

```
ws://127.0.0.1:3030/ws/executions
```

### 메시지 형식

서버로부터 수신하는 메시지는 체결이 발생할 때마다 전송됩니다:

```json
{
  "exec_id": "e1b724c2-5e61-4aba-8b8a-47d8a5a4f111",
  "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454",
  "symbol": "BTC-KRW",
  "side": "Buy",
  "price": 50000000,
  "quantity": 0.5,
  "fee": 0.05,
  "transaction_time": "2023-04-30T12:35:10.123Z"
}
```

### 필드 설명

| 필드             | 타입     | 설명                                 |
|-----------------|----------|--------------------------------------|
| exec_id         | string   | 체결 고유 식별자                      |
| order_id        | string   | 연관된 주문 ID                       |
| symbol          | string   | 거래 심볼 (예: "BTC-KRW")            |
| side            | string   | 거래 방향 ("Buy" 또는 "Sell")        |
| price           | number   | 체결 가격                            |
| quantity        | number   | 체결 수량                            |
| fee             | number   | 거래 수수료                          |
| transaction_time | string   | 체결 발생 시간 (ISO 8601 형식)       |

### 사용 예시 (JavaScript)

```javascript
const socket = new WebSocket('ws://127.0.0.1:3030/ws/executions');

socket.onopen = () => {
  console.log('체결 스트림에 연결되었습니다.');
};

socket.onmessage = (event) => {
  const execution = JSON.parse(event.data);
  console.log(`체결 발생: ${execution.symbol} - ${execution.side}, 가격: ${execution.price}, 수량: ${execution.quantity}`);
};

socket.onclose = () => {
  console.log('체결 스트림 연결이 종료되었습니다.');
};

socket.onerror = (error) => {
  console.error('WebSocket 오류:', error);
};
```

## 오더북 스트림

### 연결 URL

```
ws://127.0.0.1:3030/ws/orderbook/{symbol}
```

여기서 `{symbol}`은 구독하려는 거래 심볼입니다. 예: `BTC-KRW`

### 메시지 타입

1. **스냅샷 메시지**: 연결 직후 한 번 전송되는 전체 오더북 스냅샷
2. **업데이트 메시지**: 오더북 변경 시 전송되는 업데이트 정보

### 스냅샷 메시지 형식

```json
{
  "type": "orderbook_snapshot",
  "data": {
    "symbol": "BTC-KRW",
    "timestamp": 1682859310123,
    "bids": [
      { "price": 50000000, "volume": 1.5, "order_count": 3 },
      { "price": 49990000, "volume": 2.7, "order_count": 5 },
      ...
    ],
    "asks": [
      { "price": 50010000, "volume": 1.2, "order_count": 2 },
      { "price": 50020000, "volume": 3.4, "order_count": 4 },
      ...
    ]
  }
}
```

### 업데이트 메시지 형식

```json
{
  "type": "orderbook",
  "data": {
    "symbol": "BTC-KRW",
    "timestamp": 1682859320456,
    "bids": [
      { "price": 50000000, "volume": 1.2, "order_count": 2 },
      { "price": 49990000, "volume": 2.7, "order_count": 5 },
      ...
    ],
    "asks": [
      { "price": 50010000, "volume": 1.5, "order_count": 3 },
      { "price": 50020000, "volume": 3.4, "order_count": 4 },
      ...
    ]
  }
}
```

### 필드 설명

| 필드           | 타입     | 설명                                      |
|---------------|----------|-------------------------------------------|
| type          | string   | 메시지 타입 ("orderbook_snapshot" 또는 "orderbook") |
| data          | object   | 오더북 데이터                              |
| symbol        | string   | 거래 심볼                                 |
| timestamp     | number   | 타임스탬프 (밀리초 단위 UNIX 시간)          |
| bids          | array    | 매수 호가 배열 (가격 내림차순)               |
| asks          | array    | 매도 호가 배열 (가격 오름차순)               |
| price         | number   | 가격 수준                                 |
| volume        | number   | 해당 가격 수준의 총 수량                    |
| order_count   | number   | 해당 가격 수준의 주문 수                    |

### 주의사항

1. 스냅샷 메시지는 연결 직후 한 번만 수신됩니다.
2. 업데이트 메시지는 오더북 변경 시마다 수신되며, 전체 오더북 상태를 포함합니다.
3. 현재 구현에서는 델타 업데이트가 아닌 전체 상태를 전송합니다. 향후 버전에서는 대역폭 효율성을 위해 델타 업데이트를 제공할 수 있습니다.
4. 오더북 스트림은 주문 처리 파이프라인과 독립적으로 작동하므로, 일부 지연이 있을 수 있습니다.

### 사용 예시 (JavaScript)

```javascript
const symbol = 'BTC-KRW';
const socket = new WebSocket(`ws://127.0.0.1:3030/ws/orderbook/${symbol}`);

let orderbook = {
  bids: [],
  asks: []
};

socket.onopen = () => {
  console.log(`${symbol} 오더북 스트림에 연결되었습니다.`);
};

socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  if (message.type === 'orderbook_snapshot') {
    // 초기 스냅샷 처리
    orderbook = message.data;
    console.log('오더북 스냅샷 수신:', orderbook);
    displayOrderbook();
  } else if (message.type === 'orderbook') {
    // 업데이트 처리
    orderbook = message.data;
    console.log('오더북 업데이트 수신');
    displayOrderbook();
  }
};

function displayOrderbook() {
  console.log(`${symbol} 오더북 (${new Date(orderbook.timestamp).toLocaleTimeString()})`);
  
  console.log('매도 호가:');
  orderbook.asks.slice(0, 5).reverse().forEach(level => {
    console.log(`${level.price.toLocaleString()} - ${level.volume} (${level.order_count}개 주문)`);
  });
  
  console.log('매수 호가:');
  orderbook.bids.slice(0, 5).forEach(level => {
    console.log(`${level.price.toLocaleString()} - ${level.volume} (${level.order_count}개 주문)`);
  });
}

socket.onclose = () => {
  console.log(`${symbol} 오더북 스트림 연결이 종료되었습니다.`);
};

socket.onerror = (error) => {
  console.error('WebSocket 오류:', error);
};
```

## 릴레이 서버 (WebSocket 구독 관리)

릴레이 서버는 오더북 데이터를 클라이언트 UI에 효율적으로 중계하는 역할을 담당합니다. 이 서버는 주문 처리 파이프라인과 독립적으로 작동하므로, 많은 수의 UI 클라이언트 연결로 인한 부하가 핵심 거래 기능에 영향을 주지 않습니다.

### 주요 기능

1. **구독 관리**: 클라이언트의 심볼별 구독 요청 처리
2. **오더북 상태 모니터링**: 매칭 엔진의 오더북 상태 변화 감지
3. **효율적인 데이터 전달**: 구독 중인 클라이언트에게만 관련 데이터 전송

### 클라이언트 명령어

릴레이 서버는 클라이언트의 다음 명령어를 처리합니다:

1. **구독 요청**: `subscribe:{symbol}`
2. **구독 해제**: `unsubscribe:{symbol}`
3. **스냅샷 요청**: `snapshot:{symbol}`

예시:
- `subscribe:BTC-KRW` - BTC-KRW 오더북 구독
- `unsubscribe:BTC-KRW` - BTC-KRW 오더북 구독 해제
- `snapshot:BTC-KRW` - BTC-KRW 오더북 스냅샷 요청

## 성능 및 확장성 고려사항

### 연결 제한

현재 구현에서는 연결 수 제한이 명시적으로 설정되어 있지 않지만, 프로덕션 환경에서는 다음과 같은 제한을 고려해야 합니다:
- 클라이언트당 최대 연결 수
- IP 주소당 최대 연결 수
- 서버 전체 최대 연결 수

### 데이터 압축

대역폭 사용량을 줄이기 위해 다음과 같은 최적화를 고려할 수 있습니다:
- WebSocket 메시지 압축 (RFC 7692)
- 델타 기반 업데이트 (전체 오더북 대신 변경된 부분만 전송)
- 이진 메시지 형식 (JSON 대신 Protocol Buffers, MessagePack 등)

### 부하 분산

많은 수의 클라이언트를 지원하기 위해 다음과 같은 접근 방식을 사용할 수 있습니다:
- WebSocket 연결을 위한 별도의 서버 풀
- 심볼별 샤딩 (특정 심볼 그룹을 처리하는 전용 서버)
- 메시지 브로커 사용 (Redis PubSub, Kafka 등)

## 연결 관리

### 연결 유지 (Keepalive)

WebSocket 연결은 일정 시간 동안 활동이 없을 경우 중간 프록시나 방화벽에 의해 종료될 수 있습니다. 이를 방지하기 위해:

1. 클라이언트는 주기적으로 핑(ping) 프레임을 전송할 수 있습니다.
2. 서버는 클라이언트의 핑에 대해 퐁(pong) 프레임으로 응답합니다.

대부분의 WebSocket 클라이언트 라이브러리는 이러한 핑/퐁 메커니즘을 자동으로 처리합니다.

### 재연결 전략

클라이언트 측에서는 연결이 끊어졌을 경우를 대비한 재연결 전략을 구현하는 것이 권장됩니다:

1. 지수 백오프 알고리즘을 사용하여 재시도 간격을 점진적으로 늘림
2. 최대 재시도 횟수 설정
3. 연결이 성공적으로 복구되면 필요한 구독 다시 요청

### 재연결 예시 (JavaScript)

```javascript
class WebSocketClient {
  constructor(url, options = {}) {
    this.url = url;
    this.options = {
      reconnectAttempts: 10,
      initialReconnectDelay: 1000,
      maxReconnectDelay: 30000,
      ...options
    };
    this.reconnectAttempt = 0;
    this.connect();
  }
  
  connect() {
    this.socket = new WebSocket(this.url);
    
    this.socket.onopen = () => {
      console.log('WebSocket 연결 성공');
      this.reconnectAttempt = 0;
      if (this.options.onOpen) {
        this.options.onOpen();
      }
    };
    
    this.socket.onmessage = (event) => {
      if (this.options.onMessage) {
        this.options.onMessage(event);
      }
    };
    
    this.socket.onclose = () => {
      console.log('WebSocket 연결 종료');
      this.reconnect();
    };
    
    this.socket.onerror = (error) => {
      console.error('WebSocket 오류:', error);
      if (this.options.onError) {
        this.options.onError(error);
      }
    };
  }
  
  reconnect() {
    if (this.reconnectAttempt >= this.options.reconnectAttempts) {
      console.log('최대 재연결 시도 횟수 초과');
      if (this.options.onReconnectFailed) {
        this.options.onReconnectFailed();
      }
      return;
    }
    
    this.reconnectAttempt++;
    const delay = Math.min(
      this.options.initialReconnectDelay * Math.pow(1.5, this.reconnectAttempt - 1),
      this.options.maxReconnectDelay
    );
    
    console.log(`${delay}ms 후 재연결 시도 (${this.reconnectAttempt}/${this.options.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
      if (this.options.onReconnect) {
        this.options.onReconnect(this.reconnectAttempt);
      }
    }, delay);
  }
  
  send(data) {
    if (this.socket.readyState === WebSocket.OPEN) {
      this.socket.send(data);
      return true;
    }
    return false;
  }
  
  close() {
    this.socket.close();
  }
}

// 사용 예시
const orderbookClient = new WebSocketClient(`ws://127.0.0.1:3030/ws/orderbook/BTC-KRW`, {
  onOpen: () => {
    console.log('오더북 연결 성공');
  },
  onMessage: (event) => {
    const data = JSON.parse(event.data);
    console.log('오더북 데이터 수신:', data.type);
  },
  onReconnect: (attempt) => {
    console.log(`오더북 재연결 시도 ${attempt}`);
  }
});
```
