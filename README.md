# xTraderz

빠르고 안정적인 트레이딩 시스템 API 서버

## 개요

xTraderz는 고성능 트레이딩 엔진을 제공하는 Rust 기반 API 서버입니다. 주문 처리, 실행 및 관리를 위한 강력한 백엔드 시스템으로 설계되었습니다.

## 기능

- 주문 요청 및 처리 (매수/매도)
- 주문 실행 메커니즘
- 상태 확인 엔드포인트
- 견고한 에러 처리

## 기술 스택

- **언어**: Rust
- **웹 프레임워크**: Axum
- **비동기 런타임**: Tokio
- **로깅/트레이싱**: tracing, tracing-subscriber
- **미들웨어**: Tower, Tower-HTTP

## API 엔드포인트

| 엔드포인트 | 메서드 | 설명 |
|------------|--------|------|
| `/orders`  | POST   | 새로운 거래 주문 추가 |
| `/execute` | POST   | 대기 중인 주문 실행 |
| `/health`  | GET    | API 서버 상태 확인 |

## 시작하기

### 사전 요구사항

- Rust 및 Cargo (최신 안정 버전)

### 설치 및 실행

1. 저장소 클론
   ```bash
   git clone https://github.com/yourusername/xtraderz.git
   cd xtraderz
   ```

2. 빌드
   ```bash
   cargo build --release
   ```

3. 실행
   ```bash
   cargo run --release
   ```

서버는 기본적으로 `http://127.0.0.1:3030`에서 실행됩니다.

## 사용 예시

### 새 주문 추가

```bash
curl -X POST http://localhost:3030/orders \
  -H "Content-Type: application/json" \
  -d '{
    "symbol": "BTCUSD",
    "quantity": 0.5,
    "price": 45000,
    "side": "buy",
    "order_type": "limit"
  }'
```

### 주문 실행

```bash
curl -X POST http://localhost:3030/execute
```

### 상태 확인

```bash
curl http://localhost:3030/health
```

## 성능 최적화

xTraderz는 다음과 같은 성능 최적화를 통해 빠른 실행 시간을 보장합니다:

- Axum의 비동기 처리 활용
- Tower 미들웨어를 통한 타임아웃 및 오류 처리
- 트레이싱 레이어를 통한 성능 모니터링

## 기여하기

1. 이 저장소를 포크합니다
2. 기능 브랜치를 생성합니다 (`git checkout -b feature/amazing-feature`)
3. 변경사항을 커밋합니다 (`git commit -m 'Add some amazing feature'`)
4. 브랜치에 푸시합니다 (`git push origin feature/amazing-feature`)
5. Pull Request를 생성합니다

## 라이선스

[MIT](LICENSE)

## 연락처

프로젝트 관리자: HAMA - hama@example.com

프로젝트 링크: [https://github.com/yourusername/xtraderz](https://github.com/yourusername/xtraderz)